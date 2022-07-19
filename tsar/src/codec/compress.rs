use std::io::Write;

use super::Codec;
use crate::result::Result;

pub enum Compress {
    Zstd(i32),
}

impl Codec for Compress {
    fn encode<'a, I>(&self, data: I, out: &mut super::BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        out.reset(data.len());
        for (i, o) in data.zip(out) {
            match self {
                &Compress::Zstd(level) => {
                    let mut z = zstd::Encoder::new(o, level)?;
                    z.write_all(i)?;
                    z.finish()?;
                }
            }
        }
        Ok(())
    }

    fn decode<'a, I>(&self, data: I, out: &mut super::BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        out.reset(data.len());
        for (i, o) in data.zip(out) {
            match self {
                &Compress::Zstd(_) => {
                    let i = std::io::Cursor::new(i);
                    let mut z = zstd::Decoder::new(i)?;
                    std::io::copy(&mut z, o)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::BufferList;

    #[test]
    fn zstd_compress() {
        let mut out = BufferList::new();
        let mut decomp = BufferList::new();
        Compress::Zstd(9)
            .encode(["hello world".as_bytes()], &mut out)
            .unwrap();
        Compress::Zstd(9)
            .decode(out.iter_slice(), &mut decomp)
            .unwrap();
        assert_eq!(decomp[0], "hello world".as_bytes());
    }
}
