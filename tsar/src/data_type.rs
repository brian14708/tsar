use crate::pb::tsar as pb;

use core::cmp::Ordering;
use num_traits::cast::AsPrimitive;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum DataType {
    Byte,
    Float32,
    Float64,
    Float16,
    Bfloat16,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
}

impl From<DataType> for pb::DataType {
    fn from(d: DataType) -> Self {
        match d {
            DataType::Byte => pb::DataType::BYTE,
            DataType::Float32 => pb::DataType::FLOAT32,
            DataType::Float64 => pb::DataType::FLOAT64,
            DataType::Float16 => pb::DataType::FLOAT16,
            DataType::Bfloat16 => pb::DataType::BFLOAT16,
            DataType::Int8 => pb::DataType::INT8,
            DataType::Uint8 => pb::DataType::UINT8,
            DataType::Int16 => pb::DataType::INT16,
            DataType::Uint16 => pb::DataType::UINT16,
            DataType::Int32 => pb::DataType::INT32,
            DataType::Uint32 => pb::DataType::UINT32,
            DataType::Int64 => pb::DataType::INT64,
            DataType::Uint64 => pb::DataType::UINT64,
        }
    }
}

macro_rules! diff_float {
    ($ty:ty, $src:expr, $targ:expr) => {{
        const N: usize = std::mem::size_of::<$ty>();
        #[allow(clippy::modulo_one)]
        if $src.len() % N != 0 {
            return None;
        }
        let mut err = f64::default();
        for (src, targ) in $src.chunks_exact(N).zip($targ.chunks_exact(N)) {
            let src: f64 = <$ty>::from_le_bytes(src.try_into().unwrap()).as_();
            let targ: f64 = <$ty>::from_le_bytes(targ.try_into().unwrap()).as_();
            let num = match targ.partial_cmp(&src) {
                Some(Ordering::Equal) => continue,
                Some(Ordering::Less) => (src - targ),
                Some(Ordering::Greater) => (targ - src),
                None => return None,
            };
            let denom = src.abs().max(targ.abs());
            if err * denom < num {
                err = num / denom;
            }
        }
        Some(err)
    }};
}

macro_rules! diff_int {
    ($ty:ty, $src:expr, $targ:expr) => {{
        const N: usize = std::mem::size_of::<$ty>();
        #[allow(clippy::modulo_one)]
        if $src.len() % N != 0 {
            return None;
        }
        Some($src.chunks_exact(N).zip($targ.chunks_exact(N)).fold(
            f64::default(),
            |err, (src, targ)| {
                let src = <$ty>::from_le_bytes(src.try_into().unwrap());
                let targ = <$ty>::from_le_bytes(targ.try_into().unwrap());
                let num: f64 = match targ.cmp(&src) {
                    Ordering::Equal => return err,
                    Ordering::Less => (src - targ).as_(),
                    Ordering::Greater => (targ - src).as_(),
                };
                let denom: f64 = src.abs().max(targ.abs()).as_();
                if err * denom < num {
                    num / denom
                } else {
                    err
                }
            },
        ))
    }};

    (unsigned, $ty:ty, $src:expr, $targ:expr) => {{
        const N: usize = std::mem::size_of::<$ty>();
        #[allow(clippy::modulo_one)]
        if $src.len() % N != 0 {
            return None;
        }
        Some($src.chunks_exact(N).zip($targ.chunks_exact(N)).fold(
            f64::default(),
            |err, (src, targ)| {
                let src = <$ty>::from_le_bytes(src.try_into().unwrap());
                let targ = <$ty>::from_le_bytes(targ.try_into().unwrap());
                let num: f64 = match targ.cmp(&src) {
                    Ordering::Equal => return err,
                    Ordering::Less => (src - targ).as_(),
                    Ordering::Greater => (targ - src).as_(),
                };
                let denom: f64 = src.max(targ).as_();
                if err * denom < num {
                    num / denom
                } else {
                    err
                }
            },
        ))
    }};
}

impl DataType {
    pub fn relative_error(&self, src: &[u8], targ: &[u8]) -> Option<f64> {
        if src.len() != targ.len() {
            return None;
        }
        if src.is_empty() {
            return Some(0.0);
        }

        match self {
            DataType::Byte => {
                const N: usize = std::mem::size_of::<u64>();
                let src_it = src.chunks_exact(N);
                let targ_it = targ.chunks_exact(N);
                let mut err = src_it
                    .remainder()
                    .iter()
                    .zip(targ_it.remainder().iter())
                    .map(|(src, targ)| (targ ^ src).count_ones())
                    .sum::<u32>() as f64;
                err += src_it
                    .zip(targ_it)
                    .map(|(src, targ)| {
                        let src = <u64>::from_le_bytes(src.try_into().unwrap());
                        let targ = <u64>::from_le_bytes(targ.try_into().unwrap());
                        (targ ^ src).count_ones()
                    })
                    .sum::<u32>() as f64;
                Some(err)
            }
            DataType::Float32 => diff_float!(f32, src, targ),
            DataType::Float64 => diff_float!(f64, src, targ),
            DataType::Float16 => diff_float!(half::f16, src, targ),
            DataType::Bfloat16 => diff_float!(half::bf16, src, targ),
            DataType::Int8 => diff_int!(i8, src, targ),
            DataType::Uint8 => diff_int!(unsigned, u8, src, targ),
            DataType::Int16 => diff_int!(i16, src, targ),
            DataType::Uint16 => diff_int!(unsigned, u16, src, targ),
            DataType::Int32 => diff_int!(i32, src, targ),
            DataType::Uint32 => diff_int!(unsigned, u32, src, targ),
            DataType::Int64 => diff_int!(i64, src, targ),
            DataType::Uint64 => diff_int!(unsigned, u64, src, targ),
        }
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{LittleEndian, WriteBytesExt};

    use super::*;

    #[test]
    fn byte_diff() {
        let src = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        let targ = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        assert_eq!(DataType::Byte.relative_error(&src, &targ), Some(0.0));
        let targ = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        assert_eq!(DataType::Byte.relative_error(&src, &targ), None);
        let targ = vec![0x00, 0x01, 0x02, 0x00, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00];
        assert_eq!(DataType::Byte.relative_error(&src, &targ), Some(4.0));
    }

    #[test]
    fn f32_diff() {
        let write = |f: &[f32]| {
            let mut buf = vec![];
            for &f in f {
                buf.write_f32::<LittleEndian>(f).unwrap();
            }
            buf
        };
        let src = write(&[0.0, 1.0, 2.0, -3.0, 4.0, 5.0, 6.0]);
        let targ = write(&[0.0, 1.0, 2.0, -3.0, 4.0, 5.0, 6.0]);
        assert_eq!(DataType::Float32.relative_error(&src, &targ), Some(0.0));
        let targ = write(&[0.0, 1.0, 2.0, -3.0, 4.0, 5.0]);
        assert_eq!(DataType::Float32.relative_error(&src, &targ), None);
        let targ = write(&[0.0, 1.0, 2.0, -2.0, 4.0, 5.0, 3.0]);
        assert_eq!(DataType::Float32.relative_error(&src, &targ), Some(0.5));
    }

    #[test]
    fn f64_diff() {
        let write = |f: &[f64]| {
            let mut buf = vec![];
            for &f in f {
                buf.write_f64::<LittleEndian>(f).unwrap();
            }
            buf
        };
        let src = write(&[1.0, 2.0, -3.0, 4.0, 5.0, 6.0]);
        let targ = write(&[1.0, 2.0, -3.0, 4.0, 5.0, 6.0]);
        assert_eq!(DataType::Float64.relative_error(&src, &targ), Some(0.0));
        let targ = write(&[1.0, 2.0, -3.0, 4.0, 5.0]);
        assert_eq!(DataType::Float64.relative_error(&src, &targ), None);
        let targ = write(&[1.0, 2.0, -1.0, 4.0, 5.0, 3.0]);
        assert_eq!(
            DataType::Float64.relative_error(&src, &targ),
            Some(2.0 / 3.0)
        );
    }

    #[test]
    fn i64_diff() {
        let write = |f: &[i64]| {
            let mut buf = vec![];
            for &f in f {
                buf.write_i64::<LittleEndian>(f).unwrap();
            }
            buf
        };
        let src = write(&[1, 2, 3, 4, 5, 6]);
        let targ = write(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(DataType::Int64.relative_error(&src, &targ), Some(0.0));
        let targ = write(&[1, 2, 3, 4, 5]);
        assert_eq!(DataType::Int64.relative_error(&src, &targ), None);
        let targ = write(&[1, 2, 1, 4, 5, 3]);
        assert_eq!(DataType::Int64.relative_error(&src, &targ), Some(2.0 / 3.0));
    }

    #[test]
    fn u64_diff() {
        let write = |f: &[u64]| {
            let mut buf = vec![];
            for &f in f {
                buf.write_u64::<LittleEndian>(f).unwrap();
            }
            buf
        };
        let src = write(&[1, 2, 3, 4, 5, 6]);
        let targ = write(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(DataType::Uint64.relative_error(&src, &targ), Some(0.0));
        let targ = write(&[1, 2, 3, 4, 5]);
        assert_eq!(DataType::Uint64.relative_error(&src, &targ), None);
        let targ = write(&[1, 2, 1, 4, 5, 3]);
        assert_eq!(
            DataType::Uint64.relative_error(&src, &targ),
            Some(2.0 / 3.0)
        );
    }
}
