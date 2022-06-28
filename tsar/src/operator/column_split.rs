use half::bf16;

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
        (
            [((b >> 23) & 0xff) as u8],
            (((b & 0x80000000) >> 8) | (b & 0x7fffff)).to_le_bytes()[..3]
                .try_into()
                .unwrap(),
        )
    }

    fn from_split_bits((e, m): ([u8; 1], [u8; 3])) -> Self {
        let e = e[0] as u32;
        let m = u32::from_le_bytes([m[0], m[1], m[2], 0]);
        f32::from_bits(e << 23 | (m & 0x7fffff) | (m & 0x800000) << 8)
    }
}

impl SplitFloat for f64 {
    type Output = ([u8; 2], [u8; 7]);

    fn to_split_bits(self) -> ([u8; 2], [u8; 7]) {
        let b = self.to_bits();
        (
            (((b >> 52) & 0x7ff) as u16).to_le_bytes(),
            (((b & 0x8000000000000000) >> 11) | (b & 0xfffffffffffff)).to_le_bytes()[..7]
                .try_into()
                .unwrap(),
        )
    }

    fn from_split_bits((e, m): ([u8; 2], [u8; 7])) -> Self {
        let e = u16::from_le_bytes(e) as u64;
        let m = u64::from_le_bytes([m[0], m[1], m[2], m[3], m[4], m[5], m[6], 0]);
        f64::from_bits((e << 52) | (m & 0xfffffffffffff) | (m & 0x10000000000000) << 11)
    }
}

impl SplitFloat for half::bf16 {
    type Output = ([u8; 1], [u8; 1]);

    fn to_split_bits(self) -> ([u8; 1], [u8; 1]) {
        let b = self.to_bits();
        (
            [((b >> 7) & 0xff) as u8],
            [(((b & 0x8000) >> 8) | (b & 0x7f)) as u8],
        )
    }

    fn from_split_bits((e, m): ([u8; 1], [u8; 1])) -> Self {
        let m = m[0] as u16;
        let e = e[0] as u16;
        bf16::from_bits(e << 7 | (m & 0x7f) | (m & 0x80) << 8)
    }
}

pub(crate) struct ColumnarSplit<'p> {
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
        let mut buf_0: [u8; 512] = [0; 512];
        let mut buf_1: [u8; 512] = [0; 512];
        let mut offset_0 = 0;
        let mut offset_1 = 0;
        $in.chunks_exact(N).for_each(|m| {
            let curr = <$src>::from_le_bytes(m.try_into().unwrap());
            let result = curr.to_split_bits();
            if offset_0 + result.0.len() >= buf_0.len() {
                let l = $out_0.len() + offset_0;
                $out_0.extend(&buf_0);
                $out_0.drain(l..);
                offset_0 = 0;
            }
            if offset_1 + result.1.len() >= buf_1.len() {
                let l = $out_1.len() + offset_1;
                $out_1.extend(&buf_1);
                $out_1.drain(l..);
                offset_1 = 0;
            }
            buf_0[offset_0..offset_0 + result.0.len()].copy_from_slice(&result.0);
            buf_1[offset_1..offset_1 + result.1.len()].copy_from_slice(&result.1);
            offset_0 += result.0.len();
            offset_1 += result.1.len();
        });
        $out_0.extend(&buf_0[..offset_0]);
        $out_1.extend(&buf_1[..offset_1]);
    }};
}

impl Operator for ColumnarSplit<'_> {
    fn num_output_buffers(&self) -> usize {
        self.parent.num_output_buffers()
            * match self.mode {
                ColumnarSplitMode::Bfloat16 => 2,
                ColumnarSplitMode::Float32 => 2,
                ColumnarSplitMode::Float64 => 2,
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
            let b = bf16::from_split_bits(o);
            assert_delta!(f, b);
        }
    }
}
