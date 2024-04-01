use std::{
    collections::HashMap,
    fs,
    io::{self, Seek},
    path::{Path, PathBuf},
    sync,
};

use pyo3::prelude::*;
use rayon::prelude::*;

#[pyclass(module = "tsar.tsar")]
struct Writer {
    w: tsar::Builder<std::fs::File>,
}

#[pymethods]
impl Writer {
    #[new]
    fn new(dst: &str) -> PyResult<Self> {
        Ok(Self {
            w: tsar::Builder::new(std::fs::File::create(dst)?),
        })
    }

    pub fn write_file(&mut self, name: String, d: &[u8]) -> PyResult<()> {
        self.w.add_file(name, std::io::Cursor::new(d)).unwrap();
        Ok(())
    }

    pub fn write_blob(
        &mut self,
        ty: &str,
        name: &str,
        data: &[u8],
        dims: Vec<usize>,
        error_limit: f64,
        target_file: Option<(String, u64)>,
    ) -> PyResult<()> {
        let opt = tsar::BlobWriteOption {
            error_limit,
            target_file,
        };
        let ty = match ty {
            "f32" => Some(tsar::DataType::Float32),
            "f64" => Some(tsar::DataType::Float64),
            "f16" => Some(tsar::DataType::Float16),
            "bf16" => Some(tsar::DataType::Bfloat16),
            "i8" => Some(tsar::DataType::Int8),
            "u8" => Some(tsar::DataType::Uint8),
            "i16" => Some(tsar::DataType::Int16),
            "u16" => Some(tsar::DataType::Uint16),
            "i32" => Some(tsar::DataType::Int32),
            "u32" => Some(tsar::DataType::Uint32),
            "i64" => Some(tsar::DataType::Int64),
            "u64" => Some(tsar::DataType::Uint64),
            _ => None,
        };

        match ty {
            Some(ty) => self.w.add_blob(name, data, ty, &dims, opt),
            _ => self
                .w
                .add_blob(name, data, tsar::DataType::Byte, &[data.len()], opt),
        }
        .unwrap();
        Ok(())
    }

    pub fn close(&mut self) -> PyResult<()> {
        self.w.finish().unwrap();
        Ok(())
    }
}

#[pyclass(module = "tsar.tsar")]
struct Reader {
    r: tsar::Archive<std::fs::File>,
    lk: sync::Mutex<()>,
}

#[pymethods]
impl Reader {
    #[new]
    fn new(src: &str) -> PyResult<Self> {
        Ok(Self {
            r: tsar::Archive::new(std::fs::File::open(src)?).unwrap(),
            lk: sync::Mutex::new(()),
        })
    }

    fn extract_files(&mut self, dst: &str) -> PyResult<()> {
        let files = self
            .r
            .file_names()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();
        let base = PathBuf::from(dst);
        for f in files.iter() {
            write_all(base.join(f), self.r.file_by_name(f).unwrap())?;
        }
        Ok(())
    }

    fn extract_blobs(&mut self, dst: &str) -> PyResult<()> {
        let base = PathBuf::from(dst);
        let mut blobs = self
            .r
            .blob_names()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|b| self.r.blob_by_name(b))
            .collect::<tsar::Result<Vec<_>>>()
            .unwrap();

        let mut m = HashMap::<&str, u64>::new();

        for (k, v) in blobs.iter().flat_map(|b| {
            if let Some((target_file, offset)) = b.target_file() {
                Some((
                    target_file,
                    offset + b.byte_len().expect("unknown blob length") as u64,
                ))
            } else {
                None
            }
        }) {
            let vv = m.entry(k).or_default();
            *vv = v.max(*vv);
        }
        for (k, v) in m.iter() {
            create_file(base.join(k), *v)?;
        }

        blobs.par_iter_mut().for_each(|b| {
            if let Some((target_file, offset)) = b.target_file() {
                let mut tmp = Vec::<u8>::new();
                let p = base.join(target_file);
                std::io::copy(b, &mut tmp).unwrap();

                let _lk = self.lk.lock();
                write_to(p, std::io::Cursor::new(tmp), offset).unwrap();
            }
        });
        Ok(())
    }
}

fn write_all(p: impl AsRef<Path>, mut r: impl io::Read) -> std::io::Result<()> {
    let outpath = p.as_ref();
    if let Some(p) = outpath.parent() {
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
    }
    let mut outfile = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(outpath)?;
    io::copy(&mut r, &mut outfile)?;
    Ok(())
}

fn write_to(p: impl AsRef<Path>, mut r: impl io::Read, offset: u64) -> std::io::Result<()> {
    let outpath = p.as_ref();
    if let Some(p) = outpath.parent() {
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
    }
    let mut outfile = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(outpath)?;
    if offset > 0 {
        outfile.seek(io::SeekFrom::Start(offset))?;
    }
    io::copy(&mut r, &mut outfile)?;
    Ok(())
}

fn create_file(p: impl AsRef<Path>, sz: u64) -> std::io::Result<()> {
    let outpath = p.as_ref();
    if let Some(p) = outpath.parent() {
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
    }
    let outfile = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(outpath)?;
    outfile.set_len(sz)?;
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "tsar")]
fn tsar_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Writer>()?;
    m.add_class::<Reader>()?;
    Ok(())
}
