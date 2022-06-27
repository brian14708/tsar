use half::bf16;

pub trait SplitFloat<E, M> {
    fn to_split_bits(self) -> (E, M);
    fn from_split_bits(_: E, _: M) -> Self;
}

impl SplitFloat<u8, u32> for f32 {
    fn to_split_bits(self) -> (u8, u32) {
        let b = self.to_bits();
        (
            ((b >> 23) & 0xff) as u8,
            ((b & 0x80000000) >> 8) | (b & 0x7fffff),
        )
    }

    fn from_split_bits(e: u8, m: u32) -> Self {
        let e = e as u32;
        f32::from_bits(e << 23 | (m & 0x7fffff) | (m & 0x800000) << 8)
    }
}

impl SplitFloat<u16, u64> for f64 {
    fn to_split_bits(self) -> (u16, u64) {
        let b = self.to_bits();
        (
            ((b >> 52) & 0x7ff) as u16,
            ((b & 0x8000000000000000) >> 11) | (b & 0xfffffffffffff),
        )
    }

    fn from_split_bits(e: u16, m: u64) -> Self {
        let e = e as u64;
        f64::from_bits((e << 52) | (m & 0xfffffffffffff) | (m & 0x10000000000000) << 11)
    }
}

impl SplitFloat<u8, u8> for half::bf16 {
    fn to_split_bits(self) -> (u8, u8) {
        let b = self.to_bits();
        (
            ((b >> 7) & 0xff) as u8,
            (((b & 0x8000) >> 8) | (b & 0x7f)) as u8,
        )
    }

    fn from_split_bits(e: u8, m: u8) -> Self {
        let m = m as u16;
        let e = e as u16;
        bf16::from_bits(e << 7 | (m & 0x7f) | (m & 0x80) << 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_f32() {
        for f in [
            0.0,
            123.4,
            -123.4,
            1e30,
            1e-30,
            f32::NAN,
            f32::EPSILON,
            std::f32::consts::PI,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let (e, m) = f.to_split_bits();
            let b = f32::from_split_bits(e, m);
            if f == f {
                assert_eq!(f, b);
            }
            assert_eq!(f.to_bits(), b.to_bits());
        }
    }

    #[test]
    fn split_f64() {
        for f in [
            0.0,
            123.4,
            -123.4,
            1e30,
            1e-30,
            f64::NAN,
            f64::EPSILON,
            std::f64::consts::PI,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let (e, m) = f.to_split_bits();
            let b = f64::from_split_bits(e, m);
            if f == f {
                assert_eq!(f, b);
            }
            assert_eq!(f.to_bits(), b.to_bits());
        }
    }

    #[test]
    fn split_bf16() {
        for f in [
            bf16::from_f32(0.0),
            bf16::from_f32(123.4),
            bf16::from_f32(-123.4),
            bf16::from_f32(1e30),
            bf16::from_f32(1e-30),
            bf16::NAN,
            bf16::EPSILON,
            bf16::from_f32(std::f32::consts::PI),
            bf16::INFINITY,
            bf16::NEG_INFINITY,
        ] {
            let (e, m) = f.to_split_bits();
            let b = bf16::from_split_bits(e, m);
            if f == f {
                assert_eq!(f, b);
            }
            assert_eq!(f.to_bits(), b.to_bits());
        }
    }
}
