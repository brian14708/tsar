use crate::{
    codec::{self, BufferList, Codec},
    pb::tsar as pb,
    result::Result,
    DataType,
};

pub fn compress<'a>(
    data: &'a [u8],
    dt: DataType,
    shape: &'a [usize],
    stages: impl IntoIterator<Item = &'a pb::CompressionStage>,
    target_prec: f64,
) -> Result<(BufferList, f64)> {
    let mut out = BufferList::new();
    let mut tmp = BufferList::new();
    let stages = stages.into_iter().copied().collect::<Vec<_>>();
    for (idx, &s) in stages.iter().enumerate() {
        if idx == 0 {
            do_encode(s, [data], dt, shape, target_prec, &mut out)?;
        } else {
            do_encode(s, tmp.iter_slice(), dt, shape, target_prec, &mut out)?;
        }
        std::mem::swap(&mut out, &mut tmp);
    }
    let result = tmp.clone();
    for &s in stages.iter().rev() {
        do_decode(s, tmp.iter_slice(), dt, shape, &mut out)?;
        std::mem::swap(&mut out, &mut tmp);
    }
    let err = dt.max_difference(data, tmp.iter().next().unwrap()).unwrap();
    Ok((result, err))
}

fn do_encode<'a, I>(
    stage: pb::CompressionStage,
    data: I,
    dt: DataType,
    shape: &'a [usize],
    target_prec: f64,
    out: &mut BufferList,
) -> Result<()>
where
    I: IntoIterator<Item = &'a [u8]>,
    I::IntoIter: ExactSizeIterator,
{
    match stage {
        pb::CompressionStage::INVALID_STAGE => todo!(),
        pb::CompressionStage::ZSTD => codec::Compress::Zstd(9).encode(data, out),
        pb::CompressionStage::CONVERT_FLOAT32_TO_BFLOAT16 => {
            codec::Convert::Float32ToBfloat16.encode(data, out)
        }
        pb::CompressionStage::CONVERT_FLOAT64_TO_BFLOAT16 => {
            codec::Convert::Float64ToBfloat16.encode(data, out)
        }
        pb::CompressionStage::CONVERT_FLOAT64_TO_FLOAT32 => {
            codec::Convert::Float64ToFloat32.encode(data, out)
        }
        pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16 => codec::Split::Bfloat16.encode(data, out),
        pb::CompressionStage::SPLIT_MANTISSA_FLOAT32 => codec::Split::Float32.encode(data, out),
        pb::CompressionStage::SPLIT_MANTISSA_FLOAT64 => codec::Split::Float64.encode(data, out),
        pb::CompressionStage::ZFP_FLOAT32_1D => {
            codec::Zfp::new(dt, 1, shape, target_prec).encode(data, out)
        }
        pb::CompressionStage::ZFP_FLOAT64_1D => {
            codec::Zfp::new(dt, 1, shape, target_prec).encode(data, out)
        }
    }
}

fn do_decode<'a, I>(
    stage: pb::CompressionStage,
    data: I,
    dt: DataType,
    shape: &'a [usize],
    out: &mut BufferList,
) -> Result<()>
where
    I: IntoIterator<Item = &'a [u8]>,
    I::IntoIter: ExactSizeIterator,
{
    match stage {
        pb::CompressionStage::INVALID_STAGE => todo!(),
        pb::CompressionStage::ZSTD => codec::Compress::Zstd(9).decode(data, out),
        pb::CompressionStage::CONVERT_FLOAT32_TO_BFLOAT16 => {
            codec::Convert::Float32ToBfloat16.decode(data, out)
        }
        pb::CompressionStage::CONVERT_FLOAT64_TO_BFLOAT16 => {
            codec::Convert::Float64ToBfloat16.decode(data, out)
        }
        pb::CompressionStage::CONVERT_FLOAT64_TO_FLOAT32 => {
            codec::Convert::Float64ToFloat32.decode(data, out)
        }
        pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16 => codec::Split::Bfloat16.decode(data, out),
        pb::CompressionStage::SPLIT_MANTISSA_FLOAT32 => codec::Split::Float32.decode(data, out),
        pb::CompressionStage::SPLIT_MANTISSA_FLOAT64 => codec::Split::Float64.decode(data, out),
        pb::CompressionStage::ZFP_FLOAT32_1D => {
            codec::Zfp::new(dt, 1, shape, 0.0).decode(data, out)
        }
        pb::CompressionStage::ZFP_FLOAT64_1D => {
            codec::Zfp::new(dt, 1, shape, 0.0).decode(data, out)
        }
    }
}
