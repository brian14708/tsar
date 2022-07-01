use crate::executor::{Buffer, Context, Operator};

#[derive(Copy, Clone)]
pub enum ColumnarSplitMode {
    // split exponents and mantissa
    Bfloat16,
    Float32,
    Float64,
}

trait SplitFloat {
    type Output;

    fn to_split_bits(self) -> Self::Output;
    fn from_split_bits(_: Self::Output) -> Self;
}

impl SplitFloat for f32 {
    type Output = ([u8; 1], [u8; 3]);

    fn to_split_bits(self) -> ([u8; 1], [u8; 3]) {
        let b = self.to_bits();
        let tmp = (((b & 0x8000_0000) >> 8) | (b & 0x007f_ffff)).to_le_bytes();
        ([(b >> 23) as u8], [tmp[0], tmp[1], tmp[2]])
    }

    fn from_split_bits((e, m): ([u8; 1], [u8; 3])) -> Self {
        let e = u32::from(e[0]);
        let m = u32::from_le_bytes([m[0], m[1], m[2], 0]);
        Self::from_bits(e << 23 | (m & 0x007f_ffff) | (m & 0x0080_0000) << 8)
    }
}

impl SplitFloat for f64 {
    type Output = ([u8; 2], [u8; 7]);

    fn to_split_bits(self) -> ([u8; 2], [u8; 7]) {
        let b = self.to_bits();
        let tmp = (((b & 0x8000_0000_0000_0000) >> 11) | (b & 0x000f_ffff_ffff_ffff)).to_le_bytes();
        (
            (((b >> 52) & 0x7ff) as u16).to_le_bytes(),
            [tmp[0], tmp[1], tmp[2], tmp[3], tmp[4], tmp[5], tmp[6]],
        )
    }

    fn from_split_bits((e, m): ([u8; 2], [u8; 7])) -> Self {
        let e = u64::from(u16::from_le_bytes(e));
        let m = u64::from_le_bytes([m[0], m[1], m[2], m[3], m[4], m[5], m[6], 0]);
        Self::from_bits((e << 52) | (m & 0x000f_ffff_ffff_ffff) | (m & 0x0010_0000_0000_0000) << 11)
    }
}

impl SplitFloat for half::bf16 {
    type Output = ([u8; 1], [u8; 1]);

    fn to_split_bits(self) -> ([u8; 1], [u8; 1]) {
        let b = self.to_bits();
        (
            [(b >> 7) as u8],
            [((b >> 8) as u8 & 0x80) | (b as u8 & 0x7f)],
        )
    }

    fn from_split_bits((e, m): ([u8; 1], [u8; 1])) -> Self {
        let m = u16::from(m[0]);
        let e = u16::from(e[0]);
        Self::from_bits(e << 7 | (m & 0x7f) | (m & 0x80) << 8)
    }
}

pub struct ColumnarSplit<'p> {
    parent: Box<dyn Operator + 'p>,
    mode: ColumnarSplitMode,
}

impl<'p> ColumnarSplit<'p> {
    pub(crate) fn new(parent: Box<dyn Operator + 'p>, mode: ColumnarSplitMode) -> Box<Self> {
        Box::new(Self { parent, mode })
    }
}

macro_rules! split_float {
    ($src:ty, $in:expr, $out_0:expr, $out_1:expr) => {{
        const N: usize = std::mem::size_of::<$src>();
        assert_eq!($in.len() % N, 0);
        let ret_example: <$src as SplitFloat>::Output = Default::default();
        $out_0.resize_with($in.len() / N * ret_example.0.len(), Default::default);
        $out_1.resize_with($in.len() / N * ret_example.1.len(), Default::default);
        $in.chunks_exact(N).enumerate().for_each(|(idx, m)| {
            let result = <$src>::from_le_bytes(m.try_into().unwrap()).to_split_bits();
            $out_0[idx * result.0.len()..(idx + 1) * result.0.len()].copy_from_slice(&result.0);
            $out_1[idx * result.1.len()..(idx + 1) * result.1.len()].copy_from_slice(&result.1);
        });
    }};
}

impl Operator for ColumnarSplit<'_> {
    fn num_output_buffers(&self) -> usize {
        self.parent.num_output_buffers()
            * match self.mode {
                ColumnarSplitMode::Bfloat16
                | ColumnarSplitMode::Float32
                | ColumnarSplitMode::Float64 => 2,
            }
    }

    fn next(&mut self, ctx: &Context, out: &mut [Buffer]) -> std::io::Result<usize> {
        let nb = self.parent.num_output_buffers();
        let mut tmp = ctx.allocate(nb);
        let n = self.parent.next(ctx, &mut tmp)?;
        for (i, buf) in tmp.iter().enumerate() {
            match self.mode {
                ColumnarSplitMode::Bfloat16 => {
                    split_float!(half::bf16, buf, out[i * 2], out[i * 2 + 1]);
                }
                ColumnarSplitMode::Float32 => {
                    split_float!(f32, buf, out[i * 2], out[i * 2 + 1]);
                }
                ColumnarSplitMode::Float64 => {
                    split_float!(f64, buf, out[i * 2], out[i * 2 + 1]);
                }
            }
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_delta;
    use crate::operator::test_util;

    #[test]
    fn split_f32() {
        for f in test_util::F32_DATA {
            let o = f.to_split_bits();
            let b = f32::from_split_bits(o);
            assert_delta!(f, b);
        }
    }

    #[test]
    fn split_f64() {
        for f in test_util::F64_DATA {
            let o = f.to_split_bits();
            let b = f64::from_split_bits(o);
            assert_delta!(f, b);
        }
    }

    #[test]
    fn split_bf16() {
        for f in test_util::BF16_DATA {
            let o = f.to_split_bits();
            let b = half::bf16::from_split_bits(o);
            assert_delta!(f, b);
        }
    }
}
