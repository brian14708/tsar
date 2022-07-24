use super::Codec;

pub enum Split {
    // split exponents and mantissa
    Bfloat16,
    Float16,
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

    fn to_split_bits(self) -> Self::Output {
        let b = self.to_bits();
        let tmp = (((b & 0x8000_0000) >> 8) | (b & 0x007f_ffff)).to_le_bytes();
        ([(b >> 23) as u8], [tmp[0], tmp[1], tmp[2]])
    }

    fn from_split_bits((e, m): Self::Output) -> Self {
        let e = u32::from(e[0]);
        let m = u32::from_le_bytes([m[0], m[1], m[2], 0]);
        Self::from_bits(e << 23 | (m & 0x007f_ffff) | (m & 0x0080_0000) << 8)
    }
}

impl SplitFloat for f64 {
    type Output = ([u8; 2], [u8; 7]);

    fn to_split_bits(self) -> Self::Output {
        let b = self.to_bits();
        let tmp = (((b & 0x8000_0000_0000_0000) >> 11) | (b & 0x000f_ffff_ffff_ffff)).to_le_bytes();
        (
            (((b >> 52) & 0x7ff) as u16).to_le_bytes(),
            [tmp[0], tmp[1], tmp[2], tmp[3], tmp[4], tmp[5], tmp[6]],
        )
    }

    fn from_split_bits((e, m): Self::Output) -> Self {
        let e = u64::from(u16::from_le_bytes(e));
        let m = u64::from_le_bytes([m[0], m[1], m[2], m[3], m[4], m[5], m[6], 0]);
        Self::from_bits((e << 52) | (m & 0x000f_ffff_ffff_ffff) | (m & 0x0010_0000_0000_0000) << 11)
    }
}

impl SplitFloat for half::bf16 {
    type Output = ([u8; 1], [u8; 1]);

    fn to_split_bits(self) -> Self::Output {
        let b = self.to_bits();
        (
            [(b >> 7) as u8],
            [((b >> 8) as u8 & 0x80) | (b as u8 & 0x7f)],
        )
    }

    fn from_split_bits((e, m): Self::Output) -> Self {
        let m = u16::from(m[0]);
        let e = u16::from(e[0]);
        Self::from_bits(e << 7 | (m & 0x7f) | (m & 0x80) << 8)
    }
}

impl SplitFloat for half::f16 {
    type Output = ([u8; 1], [u8; 2]);

    fn to_split_bits(self) -> Self::Output {
        let b = self.to_bits();
        (
            [(b >> 10) as u8 & 0x1f],
            (b & 0x3ff | (b >> 5) & 0x400).to_le_bytes(),
        )
    }

    fn from_split_bits((e, m): Self::Output) -> Self {
        let e = u16::from(e[0]);
        let m = u16::from_le_bytes(m);
        Self::from_bits(e << 10 | (m & 0x3ff) | (m & 0x400) << 5)
    }
}

macro_rules! split_float {
    ($src:ty, $in:expr, $out_0:expr, $out_1:expr) => {{
        const N: usize = std::mem::size_of::<$src>();
        assert_eq!($in.len() % N, 0);
        let ret_example: <$src as SplitFloat>::Output = Default::default();
        let l = $out_0.len();
        $out_0.reserve($in.len() / N * ret_example.0.len() - l);
        let l = $out_1.len();
        $out_1.reserve($in.len() / N * ret_example.1.len() - l);
        $in.chunks_exact(N).for_each(|m| {
            let result = <$src>::from_le_bytes(m.try_into().unwrap()).to_split_bits();
            $out_0.extend_from_slice(&result.0);
            $out_1.extend_from_slice(&result.1);
        });
    }};
}

macro_rules! merge_float {
    ($src:ty, $in_0:expr, $in_1:expr, $out:expr) => {{
        const N: usize = std::mem::size_of::<$src>();
        let ret_example: <$src as SplitFloat>::Output = Default::default();
        $out.resize_with($in_0.len() / ret_example.0.len() * N, Default::default);
        $out.chunks_exact_mut(N)
            .zip(
                $in_0
                    .chunks_exact(ret_example.0.len())
                    .zip($in_1.chunks_exact(ret_example.1.len())),
            )
            .for_each(|(m, (i0, i1))| {
                let bits: <$src as SplitFloat>::Output =
                    (i0.try_into().unwrap(), i1.try_into().unwrap());
                m.copy_from_slice(&<$src>::from_split_bits(bits).to_le_bytes());
            })
    }};
}

impl Codec for Split {
    fn encode<'a, I>(&self, data: I, out: &mut super::BufferList) -> crate::result::Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        out.reset(data.len() * 2);
        for (i, data) in data.enumerate() {
            match self {
                Split::Bfloat16 => split_float!(half::bf16, data, out[i * 2], out[i * 2 + 1]),
                Split::Float16 => split_float!(half::f16, data, out[i * 2], out[i * 2 + 1]),
                Split::Float32 => split_float!(f32, data, out[i * 2], out[i * 2 + 1]),
                Split::Float64 => split_float!(f64, data, out[i * 2], out[i * 2 + 1]),
            }
        }
        Ok(())
    }

    fn decode<'a, I>(&self, data: I, out: &mut super::BufferList) -> crate::result::Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut data = data.into_iter();
        out.reset(data.len() / 2);
        for out in out.iter_mut() {
            let in_0 = data.next().unwrap();
            let in_1 = data.next().unwrap();
            match self {
                Split::Bfloat16 => merge_float!(half::bf16, in_0, in_1, out),
                Split::Float16 => merge_float!(half::f16, in_0, in_1, out),
                Split::Float32 => merge_float!(f32, in_0, in_1, out),
                Split::Float64 => merge_float!(f64, in_0, in_1, out),
            }
        }
        Ok(())
    }
}
