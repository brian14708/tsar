use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Result, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicIsize, AtomicUsize, Ordering},
        Arc,
    },
};

use parking_lot::{Mutex, RwLock};

struct DiskMap {
    num_entries: AtomicUsize,
    path: PathBuf,
    quota_in_bytes: Arc<AtomicIsize>,
    map: Mutex<HashMap<String, Entry>>,
}

impl DiskMap {
    fn new(path: impl Into<PathBuf>, max_size_in_bytes: usize) -> Result<Self> {
        let path = path.into();
        if !path.is_dir() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "DiskMap: path is not a directory",
            ))
        } else {
            Ok(DiskMap {
                path,
                map: Mutex::new(HashMap::new()),
                quota_in_bytes: Arc::new(AtomicIsize::new(max_size_in_bytes as isize)),
                num_entries: AtomicUsize::new(0),
            })
        }
    }

    fn get(&self, key: impl AsRef<str>) -> Entry {
        let mut map = self.map.lock();
        if let Some(e) = map.get(key.as_ref()) {
            e.clone()
        } else {
            let id = self.num_entries.fetch_add(1, Ordering::Relaxed);
            let e = Entry::new(self.path.join(id.to_string()), self.quota_in_bytes.clone());
            map.insert(key.as_ref().to_string(), e.clone());
            e
        }
    }

    fn remove(&self, key: impl AsRef<str>) {
        let mut map = self.map.lock();
        map.remove(key.as_ref());
    }
}

#[derive(Clone)]
struct Entry {
    inner: Arc<RwLock<EntryInner>>,
    quota_in_bytes: Arc<AtomicIsize>,
}

impl Entry {
    fn new(path: PathBuf, quota_in_bytes: Arc<AtomicIsize>) -> Self {
        Entry {
            inner: Arc::new(RwLock::new(EntryInner {
                path,
                storage: Storage::Memory(Vec::new()),
                size: 0,
            })),
            quota_in_bytes,
        }
    }

    fn read(&self) -> Result<impl Read + '_> {
        let m = self.inner.read();
        EntryReader::new(m)
    }

    fn write(&mut self) -> Result<impl Write + '_> {
        let m = self.inner.write();
        EntryWriter::new(m, self.quota_in_bytes.clone())
    }

    fn clear(&mut self) -> Result<()> {
        let mut m = self.inner.write();
        m.size = 0;
        match &mut std::mem::replace(&mut m.storage, Storage::Memory(Vec::new())) {
            Storage::Memory(v) => {
                v.clear();
                Ok(())
            }
            Storage::Disk => std::fs::remove_file(&m.path),
        }
    }
}

enum Storage {
    Memory(Vec<u8>),
    Disk,
}

struct EntryInner {
    path: PathBuf,
    storage: Storage,
    size: usize,
}

impl EntryInner {
    fn get_path(&self) -> &Path {
        &self.path
    }
}

struct EntryWriter<E>
where
    E: DerefMut<Target = EntryInner>,
{
    entry: E,
    fd: Option<File>,
    quota_in_bytes: Arc<AtomicIsize>,
}

impl<E> EntryWriter<E>
where
    E: DerefMut<Target = EntryInner>,
{
    fn new(mut entry: E, quota_in_bytes: Arc<AtomicIsize>) -> Result<Self> {
        entry.size = 0;
        let fd = match &mut entry.storage {
            Storage::Memory(v) => {
                v.clear();
                None
            }
            Storage::Disk => Some(File::create(entry.get_path())?),
        };
        Ok(EntryWriter {
            entry,
            fd,
            quota_in_bytes,
        })
    }

    fn prepare_write(&mut self, u: usize) -> Result<()> {
        if self.fd.is_none() {
            let u = u as isize;
            let left = self.quota_in_bytes.fetch_sub(u, Ordering::Relaxed) - u;
            if left < 0 {
                self.quota_in_bytes
                    .fetch_add(self.entry.size as isize + u, Ordering::Relaxed);
                let mut fd = File::create(self.entry.get_path())?;
                match &std::mem::replace(&mut self.entry.storage, Storage::Disk) {
                    Storage::Memory(v) => fd.write_all(v)?,
                    Storage::Disk => unreachable!(),
                }
                self.fd.replace(fd);
            }
        }
        self.entry.size += u;
        Ok(())
    }
}

impl<E> Write for EntryWriter<E>
where
    E: DerefMut<Target = EntryInner>,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.prepare_write(buf.len())?;
        if let Some(fd) = &mut self.fd {
            fd.write(buf)
        } else {
            match &mut self.entry.storage {
                Storage::Memory(v) => v.write(buf),
                Storage::Disk => unreachable!(),
            }
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.prepare_write(buf.len())?;
        if let Some(fd) = &mut self.fd {
            fd.write_all(buf)
        } else {
            match &mut self.entry.storage {
                Storage::Memory(v) => v.write_all(buf),
                Storage::Disk => unreachable!(),
            }
        }
    }

    fn flush(&mut self) -> Result<()> {
        if let Some(fd) = &mut self.fd {
            fd.flush()
        } else {
            match &mut self.entry.storage {
                Storage::Memory(v) => v.flush(),
                Storage::Disk => unreachable!(),
            }
        }
    }
}

struct EntryReader<E>
where
    E: Deref<Target = EntryInner>,
{
    entry: E,
    fd: Option<File>,
    offset: usize,
}

impl<E> EntryReader<E>
where
    E: Deref<Target = EntryInner>,
{
    fn new(entry: E) -> Result<Self> {
        let (fd, offset) = match &entry.storage {
            Storage::Memory(_) => (None, 0),
            Storage::Disk => (Some(File::open(entry.get_path())?), usize::MAX),
        };
        Ok(EntryReader { entry, fd, offset })
    }
}

impl<E> Read for EntryReader<E>
where
    E: Deref<Target = EntryInner>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let Some(fd) = &mut self.fd {
            fd.read(buf)
        } else {
            match &self.entry.storage {
                Storage::Memory(v) => {
                    let sz = (&v[self.offset..]).read(buf)?;
                    self.offset += sz;
                    Ok(sz)
                }
                Storage::Disk => unreachable!(),
            }
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        if let Some(fd) = &mut self.fd {
            fd.read_to_end(buf)
        } else {
            match &self.entry.storage {
                Storage::Memory(v) => {
                    let sz = (&v[self.offset..]).read_to_end(buf)?;
                    self.offset += sz;
                    Ok(sz)
                }
                Storage::Disk => unreachable!(),
            }
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        if let Some(fd) = &mut self.fd {
            fd.read_to_string(buf)
        } else {
            match &self.entry.storage {
                Storage::Memory(v) => {
                    let sz = (&v[self.offset..]).read_to_string(buf)?;
                    self.offset += sz;
                    Ok(sz)
                }
                Storage::Disk => unreachable!(),
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if let Some(fd) = &mut self.fd {
            fd.read_exact(buf)
        } else {
            match &self.entry.storage {
                Storage::Memory(v) => (&v[self.offset..]).read_exact(buf),
                Storage::Disk => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_map() {
        let tmp_dir = tempdir::TempDir::new("diskmap").unwrap();
        let dm = DiskMap::new(tmp_dir.path(), 5).unwrap();
        let prev_cnt = std::fs::read_dir(tmp_dir.path()).unwrap().count();
        for (k, v) in [
            ("a", "b"),
            ("b", "long string"),
            ("b", "s"),
            ("a", "long string"),
            ("c", "s"),
        ] {
            let mut e = dm.get(k);
            let mut w = e.write().unwrap();
            write!(&mut w, "{}", v).unwrap();
            drop(w);
            let mut r = e.read().unwrap();
            let mut buf = String::new();
            r.read_to_string(&mut buf).unwrap();
            assert_eq!(buf, v);
            drop(r);
        }
        drop(dm);
        assert_eq!(
            2,
            std::fs::read_dir(tmp_dir.path()).unwrap().count() - prev_cnt
        );
    }
}
