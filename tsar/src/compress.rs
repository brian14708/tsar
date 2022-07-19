use crate::{
    codec::{self, BufferList, Codec},
    pb::tsar as pb,
    result::Result,
    DataType,
};

pub fn compress<'a>(
    ty: DataType,
    data: &'a [u8],
    stages: impl IntoIterator<Item = &'a pb::CompressionStage>,
) -> Result<(BufferList, f64)> {
    let mut out = BufferList::new();
    let mut tmp = BufferList::new();
    let stages = stages.into_iter().copied().collect::<Vec<_>>();
    for (idx, &s) in stages.iter().enumerate() {
        if idx == 0 {
            do_encode(s, [data], &mut out)?;
        } else {
            do_encode(s, tmp.iter_slice(), &mut out)?;
        }
        std::mem::swap(&mut out, &mut tmp);
    }
    let result = tmp.clone();
    for &s in stages.iter().rev() {
        do_decode(s, tmp.iter_slice(), &mut out)?;
        std::mem::swap(&mut out, &mut tmp);
    }
    let err = ty.relative_error(data, tmp.iter().next().unwrap()).unwrap();
    Ok((result, err))
}

fn do_encode<'a, I>(stage: pb::CompressionStage, data: I, out: &mut BufferList) -> Result<()>
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
    }
}

fn do_decode<'a, I>(stage: pb::CompressionStage, data: I, out: &mut BufferList) -> Result<()>
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
    }
}
