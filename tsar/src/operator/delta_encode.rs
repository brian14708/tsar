use crate::executor::{Buffer, Context, Operator};

#[derive(Copy, Clone)]
pub enum DeltaEncodeMode {
    DiffBfloat16,
    DiffFloat32,
    DiffFloat64,
    DiffDiffBfloat16,
    DiffDiffFloat32,
    DiffDiffFloat64,
    XorUint64,
    XorUint32,
    XorUint16,
    XorUint8,
}

macro_rules! xor_encode {
    ($ty:ty, $buf:expr, $curr_vec:expr) => {
        const N: usize = std::mem::size_of::<$ty>();
        #[allow(clippy::modulo_one)]
        {
            assert_eq!($buf.len() % N, 0);
        }
        let curr = <$ty>::from_le_bytes($curr_vec[..N].try_into().unwrap());
        let curr = $buf.chunks_exact_mut(N).fold(curr, |prev, i| {
            let curr = <$ty>::from_le_bytes(i.try_into().unwrap());
            i.copy_from_slice(&(curr ^ prev).to_le_bytes());
            curr
        });
        $curr_vec[..N].copy_from_slice(&<$ty>::to_le_bytes(curr));
    };
}

macro_rules! diff_encode {
    ($ty:ty, $buf:expr, $curr_vec:expr) => {
        const N: usize = std::mem::size_of::<$ty>();
        assert_eq!($buf.len() % N, 0);
        let curr = <$ty>::from_le_bytes($curr_vec[..N].try_into().unwrap());
        let curr = $buf.chunks_exact_mut(N).fold(curr, |prev, i| {
            let curr = <$ty>::from_le_bytes(i.try_into().unwrap());
            i.copy_from_slice(&(curr - prev).to_le_bytes());
            curr
        });
        $curr_vec[..N].copy_from_slice(&<$ty>::to_le_bytes(curr));
    };
}

macro_rules! diff2_encode {
    ($ty:ty, $buf:expr, $curr_vec:expr) => {
        const N: usize = std::mem::size_of::<$ty>();
        assert_eq!($buf.len() % N, 0);
        let curr_diff = <$ty>::from_le_bytes($curr_vec[..N].try_into().unwrap());
        let curr = <$ty>::from_le_bytes($curr_vec[N..N * 2].try_into().unwrap());
        let (curr_diff, curr) =
            $buf.chunks_exact_mut(N)
                .fold((curr_diff, curr), |(prev_diff, prev), i| {
                    let curr = <$ty>::from_le_bytes(i.try_into().unwrap());
                    i.copy_from_slice(&(curr - prev - prev_diff).to_le_bytes());
                    (curr - prev, curr)
                });
        $curr_vec[..N].copy_from_slice(&<$ty>::to_le_bytes(curr_diff));
        $curr_vec[N..N * 2].copy_from_slice(&<$ty>::to_le_bytes(curr));
    };
}

fn encode(mode: DeltaEncodeMode, buf: &mut Vec<u8>, curr_vec: &mut [u8; 16]) {
    match mode {
        DeltaEncodeMode::DiffBfloat16 => {
            diff_encode!(half::bf16, buf, curr_vec);
        }
        DeltaEncodeMode::DiffFloat32 => {
            diff_encode!(f32, buf, curr_vec);
        }
        DeltaEncodeMode::DiffFloat64 => {
            diff_encode!(f64, buf, curr_vec);
        }
        DeltaEncodeMode::DiffDiffBfloat16 => {
            diff2_encode!(half::bf16, buf, curr_vec);
        }
        DeltaEncodeMode::DiffDiffFloat32 => {
            diff2_encode!(f32, buf, curr_vec);
        }
        DeltaEncodeMode::DiffDiffFloat64 => {
            diff2_encode!(f64, buf, curr_vec);
        }
        DeltaEncodeMode::XorUint8 => {
            xor_encode!(u8, buf, curr_vec);
        }
        DeltaEncodeMode::XorUint16 => {
            xor_encode!(u16, buf, curr_vec);
        }
        DeltaEncodeMode::XorUint32 => {
            xor_encode!(u32, buf, curr_vec);
        }
        DeltaEncodeMode::XorUint64 => {
            xor_encode!(u64, buf, curr_vec);
        }
    }
}

pub struct DeltaEncode<'p> {
    parent: Box<dyn Operator + 'p>,
    mode: DeltaEncodeMode,
    curr: [u8; 16],
}

impl<'p> DeltaEncode<'p> {
    pub fn new(parent: Box<dyn Operator + 'p>, mode: DeltaEncodeMode) -> Box<Self> {
        Box::new(Self {
            parent,
            mode,
            curr: Default::default(),
        })
    }
}

impl Operator for DeltaEncode<'_> {
    fn num_output_buffers(&self) -> usize {
        self.parent.num_output_buffers()
    }

    fn next(&mut self, ctx: &Context, out: &mut [Buffer]) -> std::io::Result<usize> {
        let n = self.parent.next(ctx, out)?;
        out.iter_mut()
            .for_each(|o| encode(self.mode, o, &mut self.curr));
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_u8() {
        let mut buf = vec![1, 2, 3, 127];
        encode(DeltaEncodeMode::XorUint8, &mut buf, &mut Default::default());
        assert_eq!(buf, vec![1, 1 ^ 2, 2 ^ 3, 3 ^ 127]);
    }
}
