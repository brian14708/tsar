use crate::executor::{Context, Operator};

pub struct ReadBlock<'p, R: std::io::Read> {
    block_size: usize,
    reader: &'p mut R,
}

impl<'p, R: std::io::Read> ReadBlock<'p, R> {
    pub fn new(reader: &'p mut R, block_size: usize) -> Box<Self> {
        Box::new(Self { block_size, reader })
    }
}

impl<R: std::io::Read> Operator for ReadBlock<'_, R> {
    fn num_outputs(&self) -> usize {
        1
    }

    fn next(&mut self, _ctx: &Context, out: &mut [Vec<u8>]) -> std::io::Result<usize> {
        let buf = &mut out[0];
        buf.resize_with(self.block_size, Default::default);
        let mut n = 0;
        while n != self.block_size {
            match self.reader.read(&mut buf[n..])? {
                0 => break,
                v => n += v,
            }
        }
        buf.truncate(n);
        Ok(n)
    }
}
