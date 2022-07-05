use crate::executor::Operator;

use super::{pipe_reader::PipeReader, pipe_writer::PipeWriter};

#[derive(Copy, Clone)]
pub enum CompressMode {
    Zstd(i32),
}

pub fn new_compress<'p>(
    parent: Box<dyn Operator + 'p>,
    mode: CompressMode,
) -> Box<dyn Operator + 'p> {
    match mode {
        CompressMode::Zstd(level) => PipeWriter::new(parent, |f| {
            Box::new(zstd::Encoder::new(f, level).unwrap().auto_finish())
        }),
    }
}

pub fn new_decompress<'p>(
    parent: Box<dyn Operator + 'p>,
    mode: CompressMode,
) -> Box<dyn Operator + 'p> {
    match mode {
        CompressMode::Zstd(_) => PipeReader::new(parent, 128 * 1024, |f| {
            Box::new(zstd::Decoder::new(f.clone()).unwrap())
        }),
    }
}
