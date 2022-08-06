use std::io::{Read, Seek};

use protobuf::{CodedInputStream, Message};

use crate::{codec::BufferList, compress, paths, pb, result::Result, DataType};

pub struct Archive<R: Read + Seek> {
    z: zip::read::ZipArchive<R>,
    meta: pb::Bundle,
}

impl<R: Read + Seek> Archive<R> {
    pub fn new(reader: R) -> Result<Self> {
        let mut z = zip::read::ZipArchive::new(reader)?;
        let mut f = z.by_name(paths::BUNDLE_META_PATH)?;
        let meta = pb::Bundle::parse_from(&mut CodedInputStream::new(&mut f))?;
        drop(f);
        Ok(Self { z, meta })
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.meta.raw_files.iter().map(|f| f.name.as_str())
    }

    pub fn blob_names(&self) -> impl Iterator<Item = &str> {
        self.meta.blobs.iter().map(|f| f.name.as_str())
    }

    pub fn file_by_name(&mut self, name: impl AsRef<str>) -> Result<impl Read + '_> {
        Ok(self.z.by_name(name.as_ref())?)
    }

    pub fn blob_by_name(&mut self, name: impl AsRef<str>) -> Result<Blob> {
        let name = name.as_ref();
        let b = self
            .meta
            .blobs
            .iter()
            .find(|&b| b.name == name)
            .expect("Missing blob");

        let mut bb = BufferList::new();
        bb.reset(b.chunk_ids.len());
        for (i, c) in b.chunk_ids.iter().enumerate() {
            let mut f = self
                .z
                .by_name(&paths::chunk_path(c))
                .expect("Missing chunk");
            std::io::copy(&mut f, &mut bb[i])?;
        }
        Ok(Blob {
            meta: b.clone(),
            state: BlobState::Chunks(bb),
        })
    }
}

enum BlobState {
    Invalid,
    Chunks(BufferList),
    Uncompressed(std::io::Cursor<Vec<u8>>),
}

pub struct Blob {
    meta: pb::Blob,
    state: BlobState,
}

impl Blob {
    pub fn target_file(&self) -> Option<(&str, u64)> {
        if !self.meta.target_file_name.is_empty() {
            Some((
                &self.meta.target_file_name,
                self.meta.target_offset_in_bytes as u64,
            ))
        } else {
            None
        }
    }

    pub fn name(&self) -> &str {
        &self.meta.name
    }

    pub fn byte_len(&self) -> Option<usize> {
        let dt = self.data_type()?;
        Some(
            self.meta
                .dims
                .iter()
                .map(|s| *s as usize)
                .product::<usize>()
                * dt.byte_len(),
        )
    }

    pub fn data_type(&self) -> Option<DataType> {
        self.meta.data_type.try_into().ok()
    }

    pub fn shape(&self) -> impl IntoIterator<Item = usize> + '_ {
        self.meta.dims.iter().map(|f| *f as usize)
    }

    fn get_data(&mut self) -> std::io::Result<&mut impl std::io::Read> {
        if matches!(&mut self.state, BlobState::Chunks(_)) {
            if let BlobState::Chunks(b) = std::mem::replace(&mut self.state, BlobState::Invalid) {
                let dt = self.data_type().expect("unknown data format");
                let stages = self
                    .meta
                    .compression_stages
                    .iter()
                    .map(|e| e.unwrap())
                    .collect::<Vec<_>>();
                let d = compress::decompress(
                    b,
                    dt,
                    &self
                        .meta
                        .dims
                        .iter()
                        .map(|d| *d as usize)
                        .collect::<Vec<_>>(),
                    &stages,
                )
                .unwrap();

                self.state = BlobState::Uncompressed(std::io::Cursor::new(d));
            }
        }

        if let BlobState::Uncompressed(d) = &mut self.state {
            Ok(d)
        } else {
            todo!()
        }
    }
}

impl Read for Blob {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.get_data().unwrap().read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.get_data().unwrap().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        self.get_data().unwrap().read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.get_data().unwrap().read_exact(buf)
    }
}
