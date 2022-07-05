use std::collections::HashSet;
use std::io::{Read, Seek, Write};

use protobuf::{CodedOutputStream, EnumOrUnknown, Message};
use sha1::{Digest, Sha1};
use zip::write::FileOptions;

use crate::executor::Executable;
use crate::operator::multi_write::MultiWrite;
use crate::pb::tsar::{Blob, BlobFile, Bundle, CompressionStage, RawFile};
use crate::result::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const METADATA_FILE: &str = ".tsar-bundle";

pub struct Writer<W: Write + Seek> {
    z: zip::write::ZipWriter<W>,
    meta: Bundle,
    chunks: HashSet<String>,
}

#[derive(Default)]
pub struct WriteOption {
    pub level: i32,
    pub relative_error: f64,
}

impl<W: Write + Seek> Writer<W> {
    pub fn new(inner: W) -> Self {
        let mut z = zip::write::ZipWriter::new(inner);
        z.set_comment(format!("tsar v{}", VERSION));
        Self {
            z,
            meta: Bundle::new(),
            chunks: HashSet::new(),
        }
    }

    pub fn write_file(&mut self, name: impl Into<String>, mut reader: impl Read) -> Result<()> {
        let name = name.into();
        self.z
            .start_file(name.clone(), FileOptions::default().large_file(true))?;
        std::io::copy(&mut reader, &mut self.z)?;
        self.meta.raw_files.push(RawFile {
            name,
            ..Default::default()
        });
        Ok(())
    }

    pub fn write_blob_tensor_f32(
        &mut self,
        name: impl Into<String>,
        offset: u64,
        mut reader: impl Read,
        dims: &[usize],
        _opt: WriteOption,
    ) -> Result<()> {
        let stages = vec![
            crate::Stage::DataConvert(crate::DataConvertMode::Float32ToBfloat16),
            crate::Stage::ColumnarSplit(crate::ColumnarSplitMode::Bfloat16),
            crate::Stage::Compress(crate::CompressMode::Zstd(9)),
        ];

        let mut output = vec![];
        let sz = {
            let compressor = crate::Compressor::new(stages.iter().cloned());
            let g = compressor.build_cgraph(&mut reader);
            output.resize_with(g.num_outputs(), Vec::<u8>::new);

            let mut writer: Vec<_> = output.iter_mut().map(std::io::Cursor::new).collect();
            let mut g = MultiWrite::new(g, writer.iter_mut().map(|o| o as &mut dyn Write));
            g.execute_discard()?
        };

        let b = Blob {
            uncompressed_size_in_bytes: sz as i64,
            file_offset_in_bytes: offset as i64,
            compression_stages: [CompressionStage::CONVERT_FLOAT32_TO_BFLOAT16]
                .map(EnumOrUnknown::new)
                .into_iter()
                .collect(),
            dims: dims.iter().map(|v| *v as i64).collect(),
            ..Default::default()
        };
        self.write_chunks(name.into(), b, output.iter().map(|f| f as &[u8]))
    }

    pub fn write_blob(
        &mut self,
        _name: impl Into<String>,
        _offset: u64,
        mut _reader: impl Read,
    ) -> Result<()> {
        Ok(())
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
        mut blob: Blob,
        iter: impl IntoIterator<Item = &'a [u8]>,
    ) -> Result<()> {
        for o in iter {
            let mut hasher = Sha1::new();
            hasher.update(o);
            let result = hasher.finalize();
            let result = base64::encode_config(&result, base64::URL_SAFE);

            if !self.chunks.contains(&result) {
                self.z.start_file(
                    format!(".tsar-chunk/{}", result),
                    FileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored)
                        .large_file(true),
                )?;
                self.z.write_all(o)?;
                self.chunks.insert(result.clone());
            }
            blob.chunk_ids.push(result);
        }

        for f in self.meta.blob_files.iter_mut() {
            if f.name == name {
                f.blobs.push(blob);
                return Ok(());
            }
        }

        self.meta.blob_files.push(BlobFile {
            name,
            blobs: vec![blob],
            ..Default::default()
        });
        Ok(())
    }
}
