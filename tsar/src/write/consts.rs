use crate::{pb::tsar as pb, DataType};

macro_rules! methods {
    ($($e:expr),* $(,)*) => {
        &[
            $(&$e,)*
        ]
    };
}

pub const COMPRESS_METHOD: [(DataType, &[&[pb::CompressionStage]]); 13] = [
    (
        DataType::Float32,
        methods![
            [pb::CompressionStage::ZFP_FLOAT32_1D],
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
        methods![
            [pb::CompressionStage::ZFP_FLOAT64_1D],
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
        methods![
            [pb::CompressionStage::ZSTD],
            [
                pb::CompressionStage::SPLIT_MANTISSA_BFLOAT16,
                pb::CompressionStage::ZSTD,
            ],
        ],
    ),
    (DataType::Byte, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Float16, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Int8, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Uint8, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Int16, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Uint16, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Int32, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Uint32, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Int64, methods![[pb::CompressionStage::ZSTD],]),
    (DataType::Uint64, methods![[pb::CompressionStage::ZSTD],]),
];
