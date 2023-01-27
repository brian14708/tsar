mod consts;

use std::{
    collections::HashSet,
    io::{Read, Seek, Write},
};

use base64::Engine;
use protobuf::{CodedOutputStream, EnumOrUnknown, Message};
use sha1::{Digest, Sha1};
use zip::write::FileOptions;

use crate::{compress, paths, pb, result::Result, DataType};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Builder<W: Write + Seek> {
    z: zip::write::ZipWriter<W>,
    meta: pb::Bundle,
    chunks: HashSet<String>,
}

#[derive(Default)]
pub struct BlobWriteOption {
    pub error_limit: f64,
    pub target_file: Option<(String, u64)>,
}

impl<W: Write + Seek> Builder<W> {
    pub fn new(inner: W) -> Self {
        let mut z = zip::write::ZipWriter::new(inner);
        z.set_comment(format!("tsar v{VERSION}"));
        Self {
            z,
            meta: pb::Bundle::new(),
            chunks: HashSet::new(),
        }
    }

    pub fn add_file(&mut self, name: impl Into<String>, mut reader: impl Read) -> Result<()> {
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

    pub fn add_blob<'a>(
        &mut self,
        name: impl Into<String>,
        data: &'a [u8],
        dt: DataType,
        dims: impl IntoIterator<Item = &'a usize>,
        opt: BlobWriteOption,
    ) -> Result<()> {
        let shape = dims.into_iter().copied().collect::<Vec<_>>();
        let mut b = pb::Blob {
            name: name.into(),
            dims: shape.iter().map(|&f| f as i64).collect(),
            data_type: EnumOrUnknown::new(dt.into()),
            ..Default::default()
        };
        if let Some((f, o)) = opt.target_file {
            b.target_file_name = f;
            b.target_offset_in_bytes = o as i64;
        }

        let cand_stages = consts::COMPRESS_METHOD
            .iter()
            .find(|(t, _)| *t == dt)
            .map(|(_, m)| *m)
            .unwrap_or_default();

        let blk = &data[..(64 * 1024).min(data.len())];
        let mut sizes = cand_stages
            .iter()
            .enumerate()
            .flat_map(|(i, &stages)| -> Result<_> {
                let (r, e) = compress::compress(
                    blk,
                    dt,
                    &[blk.len() / dt.byte_len()],
                    stages,
                    opt.error_limit,
                )?;
                Ok((i, r.iter().map(Vec::len).sum::<usize>(), e))
            })
            .filter(|&(_, _, e)| e <= opt.error_limit)
            .collect::<Vec<_>>();
        sizes.sort_by_key(|(_, sz, _)| *sz);

        for (idx, _, _) in sizes {
            let stages = cand_stages[idx];
            let (output, err) = compress::compress(data, dt, &shape, stages, opt.error_limit)?;
            if err > opt.error_limit {
                continue;
            }

            b.compression_stages = cand_stages[idx]
                .iter()
                .cloned()
                .map(EnumOrUnknown::new)
                .collect();
            return self.write_chunks(b, output.iter_slice());
        }

        b.compression_stages.clear();
        self.write_chunks(b, [data])
    }

    pub fn finish(&mut self) -> Result<()> {
        self.z
            .start_file(paths::BUNDLE_META_PATH, FileOptions::default())?;
        self.meta.blobs.sort_by(|a, b| a.name.cmp(&b.name));
        // TODO check target_file contiguous
        self.meta
            .write_to(&mut CodedOutputStream::new(&mut self.z))
            .unwrap();
        self.z.finish()?;
        Ok(())
    }

    fn write_chunks<'a>(
        &mut self,
        mut blob: pb::Blob,
        iter: impl IntoIterator<Item = &'a [u8]>,
    ) -> Result<()> {
        for o in iter {
            let result = base64::prelude::BASE64_URL_SAFE.encode(Sha1::digest(o));

            if !self.chunks.contains(&result) {
                self.z.start_file(
                    paths::chunk_path(&result),
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

        self.meta.blobs.push(blob);
        Ok(())
    }
}
