use crate::{pb::tsar as pb, DataType};

macro_rules! with_default_methods {
    ($($e:expr),* $(,)*) => {
        &[
            &[pb::CompressionStage::ZSTD],
            $(&$e,)*
        ]
    };
}

pub const COMPRESS_METHOD: [(DataType, &[&[pb::CompressionStage]]); 13] = [
    (
        DataType::Float32,
        with_default_methods![
            [
                pb::CompressionStage::SPLIT_MANTISSA_FLOAT32,
                pb::CompressionStage::ZSTD,
            ],
            [
                pb::CompressionStage::CONVERT_FLOAT32_TO_BFLOAT16,
                pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16,
                pb::CompressionStage::ZSTD,
            ],
        ],
    ),
    (
        DataType::Float64,
        with_default_methods![
            [
                pb::CompressionStage::SPLIT_MANTISSA_FLOAT64,
                pb::CompressionStage::ZSTD,
            ],
            [
                pb::CompressionStage::CONVERT_FLOAT64_TO_FLOAT32,
                pb::CompressionStage::SPLIT_MANTISSA_FLOAT32,
                pb::CompressionStage::ZSTD,
            ],
            [
                pb::CompressionStage::CONVERT_FLOAT64_TO_BFLOAT16,
                pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16,
                pb::CompressionStage::ZSTD,
            ],
        ],
    ),
    (
        DataType::Bfloat16,
        with_default_methods![[
            pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16,
            pb::CompressionStage::ZSTD,
        ],],
    ),
    (DataType::Byte, with_default_methods![]),
    (DataType::Float16, with_default_methods![]),
    (DataType::Int8, with_default_methods![]),
    (DataType::Uint8, with_default_methods![]),
    (DataType::Int16, with_default_methods![]),
    (DataType::Uint16, with_default_methods![]),
    (DataType::Int32, with_default_methods![]),
    (DataType::Uint32, with_default_methods![]),
    (DataType::Int64, with_default_methods![]),
    (DataType::Uint64, with_default_methods![]),
];
