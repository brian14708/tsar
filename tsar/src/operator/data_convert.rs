use half::prelude::HalfFloatSliceExt;
use num_traits::AsPrimitive;

use crate::executor::{Buffer, Context, Operator};

#[derive(Copy, Clone)]
pub enum DataConvertMode {
    Float32ToBfloat16,
    Float64ToBfloat16,
    Float64ToFloat32,
}

macro_rules! cvt_type {
    ($src:ty, $dst:ty, $buf:expr) => {
        const NS: usize = std::mem::size_of::<$src>();
        const ND: usize = std::mem::size_of::<$dst>();
        assert_eq!($buf.len() % NS, 0);
        assert!(ND <= NS);
        let offset = (0..$buf.len()).step_by(NS).fold(0, |offset, i| {
            let curr: $dst = <$src>::from_le_bytes($buf[i..i + NS].try_into().unwrap()).as_();
            $buf[offset..offset + ND].copy_from_slice(&curr.to_le_bytes());
            offset + ND
        });
        $buf.truncate(offset);
    };
}

macro_rules! cvt_type_blk {
    ($src:ty, $dst:ty, $fun:expr, $blk_size: literal, $buf:expr) => {
        const BLK: usize = $blk_size;
        const NS: usize = std::mem::size_of::<$src>();
        const ND: usize = std::mem::size_of::<$dst>();
        const BLK_IN_BYTES: usize = BLK * NS;
        assert_eq!($buf.len() % NS, 0);

        let mut data_tmp: [$src; BLK] = Default::default();
        let mut bf16_tmp: [$dst; BLK] = Default::default();
        let blk_len = ($buf.len() / BLK_IN_BYTES) * BLK_IN_BYTES;
        let offset = (0..blk_len).step_by(BLK_IN_BYTES).fold(0, |offset, i| {
            $buf[i..i + BLK_IN_BYTES]
                .chunks_exact(NS)
                .enumerate()
                .for_each(|(i, b)| {
                    data_tmp[i] = <$src>::from_le_bytes(b.try_into().unwrap());
                });
            $fun(&mut bf16_tmp, &data_tmp);
            $buf[offset..offset + ND * BLK]
                .chunks_exact_mut(ND)
                .zip(bf16_tmp.reinterpret_cast())
                .for_each(|(buf, v)| {
                    buf.copy_from_slice(&v.to_le_bytes());
                });
            offset + ND * BLK
        });

        let offset = (blk_len..$buf.len()).step_by(NS).fold(offset, |offset, i| {
            let curr: $dst = <$src>::from_le_bytes($buf[i..i + NS].try_into().unwrap()).as_();
            $buf[offset..offset + ND].copy_from_slice(&curr.to_le_bytes());
            offset + ND
        });
        $buf.truncate(offset);
    };
}

fn encode(mode: DataConvertMode, buf: &mut Vec<u8>) {
    match mode {
        DataConvertMode::Float32ToBfloat16 => {
            cvt_type_blk!(
                f32,
                half::bf16,
                <[half::bf16]>::convert_from_f32_slice,
                16,
                buf
            );
        }
        DataConvertMode::Float64ToBfloat16 => {
            cvt_type_blk!(
                f64,
                half::bf16,
                <[half::bf16]>::convert_from_f64_slice,
                16,
                buf
            );
        }
        DataConvertMode::Float64ToFloat32 => {
            cvt_type!(f64, f32, buf);
        }
    }
}

pub struct DataConvert<'p> {
    parent: Box<dyn Operator + 'p>,
    mode: DataConvertMode,
}

impl<'p> DataConvert<'p> {
    pub(crate) fn new(parent: Box<dyn Operator + 'p>, mode: DataConvertMode) -> Box<Self> {
        Box::new(Self { parent, mode })
    }
}

impl Operator for DataConvert<'_> {
    fn num_output_buffers(&self) -> usize {
        self.parent.num_output_buffers()
    }

    fn next(&mut self, ctx: &Context, out: &mut [Buffer]) -> std::io::Result<usize> {
        let n = self.parent.next(ctx, out)?;
        out.iter_mut().for_each(|o| encode(self.mode, o.as_mut()));
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_delta;
    use crate::operator::test_util;
    use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

    #[test]
    fn f32_to_bf16() {
        let mut wtr = vec![];
        for &f in test_util::F32_DATA.iter() {
            wtr.write_f32::<LittleEndian>(f).unwrap();
        }
        encode(DataConvertMode::Float32ToBfloat16, &mut wtr);
        let mut rdr = std::io::Cursor::new(wtr);
        for &f in test_util::F32_DATA.iter() {
            let bf16 = half::bf16::from_bits(rdr.read_u16::<LittleEndian>().unwrap());
            assert_delta!(bf16, half::bf16::from_f32(f), half::bf16::default());
        }
    }

    #[test]
    fn f64_to_bf16() {
        let mut wtr = vec![];
        for &f in test_util::F64_DATA.iter() {
            wtr.write_f64::<LittleEndian>(f).unwrap();
        }
        encode(DataConvertMode::Float64ToBfloat16, &mut wtr);
        let mut rdr = std::io::Cursor::new(wtr);
        for &f in test_util::F64_DATA.iter() {
            let bf16 = half::bf16::from_bits(rdr.read_u16::<LittleEndian>().unwrap());
            assert_delta!(bf16, half::bf16::from_f64(f));
        }
    }

    #[test]
    fn f64_to_f64() {
        let mut wtr = vec![];
        for &f in test_util::F64_DATA.iter() {
            wtr.write_f64::<LittleEndian>(f).unwrap();
        }
        encode(DataConvertMode::Float64ToFloat32, &mut wtr);
        let mut rdr = std::io::Cursor::new(wtr);
        for &f in test_util::F64_DATA.iter() {
            let ff = f32::from_bits(rdr.read_u32::<LittleEndian>().unwrap());
            assert_delta!(ff, (f as f32));
        }
    }
}
