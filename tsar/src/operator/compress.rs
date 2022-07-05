use crate::executor::Operator;

use super::pipe::Pipe;

#[derive(Copy, Clone)]
pub enum CompressMode {
    Zstd(i32),
}

pub struct Compress;

impl Compress {
    pub fn new<'p>(parent: Box<dyn Operator + 'p>, mode: CompressMode) -> Box<dyn Operator + 'p> {
        match mode {
            CompressMode::Zstd(level) => Pipe::new(parent, |f| {
                Box::new(zstd::Encoder::new(f, level).unwrap().auto_finish())
            }),
        }
    }
}
