use half::prelude::HalfFloatSliceExt;
use num_traits::AsPrimitive;

use super::Codec;
use crate::result::Result;

#[derive(Copy, Clone)]
pub enum Convert {
    Float32ToBfloat16,
    Float32ToFloat16,
    Float64ToBfloat16,
    Float64ToFloat16,
    Float64ToFloat32,
}

macro_rules! cvt_type_blk {
    ($src:ty, $dst:ty, $fun:expr, $blk_size: literal, $buf:expr, $out:expr) => {
        const BLK: usize = $blk_size;
        const NS: usize = std::mem::size_of::<$src>();
        const ND: usize = std::mem::size_of::<$dst>();
        const BLK_IN_BYTES: usize = BLK * NS;
        assert_eq!($buf.len() % NS, 0);

        let l = $out.len();
        $out.reserve($buf.len() / NS * ND - l);

        let mut src_tmp: [$src; BLK] = Default::default();
        let mut dst_tmp: [$dst; BLK] = Default::default();

        let bit = $buf.chunks_exact(BLK_IN_BYTES);
        let r = bit.remainder();
        bit.for_each(|buf| {
            src_tmp
                .iter_mut()
                .zip(buf.chunks_exact(NS))
                .for_each(|(s, b)| {
                    *s = <$src>::from_le_bytes(b.try_into().unwrap());
                });
            $fun(&mut dst_tmp, &src_tmp);
            dst_tmp.iter().for_each(|v| {
                $out.extend_from_slice(&v.to_le_bytes());
            });
        });
        r.chunks_exact(NS).for_each(|buf| {
            let curr: $dst = <$src>::from_le_bytes(buf.try_into().unwrap()).as_();
            $out.extend_from_slice(&curr.to_le_bytes());
        });
    };
}

fn cvt<const N: usize, S: AsPrimitive<D>, D: Copy + 'static>(dst: &mut [D; N], src: &[S; N]) {
    *dst = src.map(S::as_);
}

impl Codec for Convert {
    fn encode<'a, I>(&self, data: I, out: &mut super::BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        out.reset(data.len());
        for (buf, out) in data.zip(out.iter_mut()) {
            match self {
                Self::Float32ToBfloat16 => {
                    cvt_type_blk!(
                        f32,
                        half::bf16,
                        <[half::bf16]>::convert_from_f32_slice,
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToBfloat16 => {
                    cvt_type_blk!(
                        f64,
                        half::bf16,
                        <[half::bf16]>::convert_from_f64_slice,
                        16,
                        buf,
                        out
                    );
                }
                Self::Float32ToFloat16 => {
                    cvt_type_blk!(
                        f32,
                        half::f16,
                        <[half::f16]>::convert_from_f32_slice,
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToFloat16 => {
                    cvt_type_blk!(
                        f64,
                        half::f16,
                        <[half::f16]>::convert_from_f64_slice,
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToFloat32 => {
                    cvt_type_blk!(f64, f32, cvt::<16, f64, f32>, 16, buf, out);
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
        for (buf, out) in data.zip(out.iter_mut()) {
            match self {
                Self::Float32ToBfloat16 => {
                    cvt_type_blk!(
                        half::bf16,
                        f32,
                        |d, s: &[half::bf16]| s.convert_to_f32_slice(d),
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToBfloat16 => {
                    cvt_type_blk!(
                        half::bf16,
                        f64,
                        |d, s: &[half::bf16]| s.convert_to_f64_slice(d),
                        16,
                        buf,
                        out
                    );
                }
                Self::Float32ToFloat16 => {
                    cvt_type_blk!(
                        half::f16,
                        f32,
                        |d, s: &[half::f16]| s.convert_to_f32_slice(d),
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToFloat16 => {
                    cvt_type_blk!(
                        half::f16,
                        f64,
                        |d, s: &[half::f16]| s.convert_to_f64_slice(d),
                        16,
                        buf,
                        out
                    );
                }
                Self::Float64ToFloat32 => {
                    cvt_type_blk!(f32, f64, cvt::<16, f32, f64>, 16, buf, out);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    use super::*;
    use crate::codec::{test_util, BufferList};

    #[test]
    fn f32_to_bf16() {
        let mut orig = vec![];
        test_util::F32_DATA.iter().for_each(|&f| {
            orig.write_f32::<LittleEndian>(f).unwrap();
        });

        let mut out_1 = BufferList::new();
        let mut out_2 = BufferList::new();
        Convert::Float32ToBfloat16
            .encode([orig.as_slice()], &mut out_1)
            .unwrap();
        Convert::Float32ToBfloat16
            .decode(out_1.iter_slice(), &mut out_2)
            .unwrap();

        let mut reader_out_1 = std::io::Cursor::new(&out_1[0]);
        let mut reader_out_2 = std::io::Cursor::new(&out_2[0]);
        for &f in test_util::F32_DATA.iter() {
            let bf16 = half::bf16::from_bits(reader_out_1.read_u16::<LittleEndian>().unwrap());
            let fp32 = reader_out_2.read_f32::<LittleEndian>().unwrap();
            assert_eq!(bf16.to_le_bytes(), half::bf16::from_f32(f).to_le_bytes());
            assert_eq!(
                fp32.to_le_bytes(),
                f32::from(half::bf16::from_f32(f)).to_le_bytes()
            );
        }
    }

    #[test]
    fn f64_to_bf16() {
        let mut orig = vec![];
        test_util::F64_DATA.iter().for_each(|&f| {
            orig.write_f64::<LittleEndian>(f).unwrap();
        });

        let mut out_1 = BufferList::new();
        let mut out_2 = BufferList::new();
        Convert::Float64ToBfloat16
            .encode([orig.as_slice()], &mut out_1)
            .unwrap();
        Convert::Float64ToBfloat16
            .decode(out_1.iter_slice(), &mut out_2)
            .unwrap();

        let mut reader_out_1 = std::io::Cursor::new(&out_1[0]);
        let mut reader_out_2 = std::io::Cursor::new(&out_2[0]);
        for &f in test_util::F64_DATA.iter() {
            let bf16 = half::bf16::from_bits(reader_out_1.read_u16::<LittleEndian>().unwrap());
            let fp64 = reader_out_2.read_f64::<LittleEndian>().unwrap();
            assert_eq!(bf16.to_le_bytes(), half::bf16::from_f64(f).to_le_bytes());
            assert_eq!(
                fp64.to_le_bytes(),
                f64::from(half::bf16::from_f64(f)).to_le_bytes()
            );
        }
    }

    #[test]
    fn f64_to_f32() {
        let mut orig = vec![];
        test_util::F64_DATA.iter().for_each(|&f| {
            orig.write_f64::<LittleEndian>(f).unwrap();
        });

        let mut out_1 = BufferList::new();
        let mut out_2 = BufferList::new();
        Convert::Float64ToFloat32
            .encode([orig.as_slice()], &mut out_1)
            .unwrap();
        Convert::Float64ToFloat32
            .decode(out_1.iter_slice(), &mut out_2)
            .unwrap();

        let mut reader_out_1 = std::io::Cursor::new(&out_1[0]);
        let mut reader_out_2 = std::io::Cursor::new(&out_2[0]);
        for &f in test_util::F64_DATA.iter() {
            let fp32 = reader_out_1.read_f32::<LittleEndian>().unwrap();
            let fp64 = reader_out_2.read_f64::<LittleEndian>().unwrap();
            assert_eq!(fp32.to_le_bytes(), (f as f32).to_le_bytes());
            assert_eq!(fp64.to_le_bytes(), (f as f32 as f64).to_le_bytes());
        }
    }
}
