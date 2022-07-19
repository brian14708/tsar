mod consts;

use std::{
    collections::HashSet,
    io::{Read, Seek, Write},
};

use protobuf::{CodedOutputStream, EnumOrUnknown, Message};
use sha1::{Digest, Sha1};
use zip::write::FileOptions;

use crate::{compress, pb::tsar as pb, result::Result, DataType};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const METADATA_FILE: &str = ".tsar/bundle";

pub struct Writer<W: Write + Seek> {
    z: zip::write::ZipWriter<W>,
    meta: pb::Bundle,
    chunks: HashSet<String>,
}

#[derive(Default)]
pub struct BlobOption {
    pub relative_error: f64,
}

impl<W: Write + Seek> Writer<W> {
    pub fn new(inner: W) -> Self {
        let mut z = zip::write::ZipWriter::new(inner);
        z.set_comment(format!("tsar v{}", VERSION));
        Self {
            z,
            meta: pb::Bundle::new(),
            chunks: HashSet::new(),
        }
    }

    pub fn write_file(&mut self, name: impl Into<String>, mut reader: impl Read) -> Result<()> {
        let name = name.into();
        self.z
            .start_file(name.clone(), FileOptions::default().large_file(true))?;
        std::io::copy(&mut reader, &mut self.z)?;
        self.meta.raw_files.push(pb::RawFile {
            name,
            ..Default::default()
        });
        Ok(())
    }

    pub fn write_blob<'a>(
        &mut self,
        name: impl Into<String>,
        offset: usize,
        data: &'a [u8],
        ty: DataType,
        dims: impl IntoIterator<Item = &'a usize>,
        opt: BlobOption,
    ) -> Result<()> {
        let mut b = pb::Blob {
            file_offset_in_bytes: offset as i64,
            dims: dims.into_iter().map(|v| *v as i64).collect(),
            data_type: EnumOrUnknown::new(ty.into()),
            ..Default::default()
        };

        let cand_stages = consts::COMPRESS_METHOD
            .iter()
            .find(|(t, _)| *t == ty)
            .map(|(_, m)| *m)
            .unwrap_or_default();

        let blk = &data[..(64 * 1024).min(data.len())];
        let mut sizes = cand_stages
            .iter()
            .enumerate()
            .map(|(i, &stages)| {
                let (r, e) = compress::compress(ty, blk, stages)?;
                Ok((i, r.iter().map(Vec::len).sum::<usize>(), e))
            })
            .filter(|o| {
                if let &Ok((_, _, e)) = o {
                    e <= opt.relative_error
                } else {
                    true
                }
            })
            .collect::<Result<Vec<_>>>()?;
        sizes.sort_by_key(|(_, sz, _)| *sz);

        for (idx, _, _) in sizes {
            let stages = cand_stages[idx];
            let (output, err) = compress::compress(ty, data, stages)?;
            if err > opt.relative_error {
                continue;
            }

            b.compression_stages = cand_stages[idx]
                .iter()
                .cloned()
                .map(EnumOrUnknown::new)
                .collect();
            return self.write_chunks(name.into(), b, output.iter_slice());
        }

        b.compression_stages.clear();
        self.write_chunks(name.into(), b, [data])
    }

    pub fn finish(&mut self) -> Result<()> {
        self.z.start_file(METADATA_FILE, FileOptions::default())?;
        self.meta.blob_files.sort_by(|a, b| a.name.cmp(&b.name));
        for f in self.meta.blob_files.iter_mut() {
            f.blobs
                .sort_by(|a, b| a.file_offset_in_bytes.cmp(&b.file_offset_in_bytes))
        }
        self.meta
            .write_to(&mut CodedOutputStream::new(&mut self.z))
            .unwrap();
        self.z.finish()?;
        Ok(())
    }

    fn write_chunks<'a>(
        &mut self,
        name: String,
        mut blob: pb::Blob,
        iter: impl IntoIterator<Item = &'a [u8]>,
    ) -> Result<()> {
        for o in iter {
            let result = base64::encode_config(&Sha1::digest(o), base64::URL_SAFE);

            if !self.chunks.contains(&result) {
                self.z.start_file(
                    format!(".tsar/chunk/{}", result),
                    FileOptions::default()
                        .compression_method(if blob.compression_stages.is_empty() {
                            // use zip compression when no custom compression stage
                            zip::CompressionMethod::DEFLATE
                        } else {
                            zip::CompressionMethod::Stored
                        })
                        .large_file(true),
                )?;
                self.z.write_all(o)?;
                self.chunks.insert(result.clone());
            }
            blob.chunk_ids.push(result);
        }

        if let Some(f) = self.meta.blob_files.iter_mut().find(|f| f.name == name) {
            f.blobs.push(blob);
        } else {
            self.meta.blob_files.push(pb::BlobFile {
                name,
                blobs: vec![blob],
                ..Default::default()
            });
        }
        Ok(())
    }
}
