use crate::{
    executor::Executable,
    executor::Operator,
    operator::{
        column_split::ColumnarSplit, data_convert::DataConvert, delta_encode::DeltaEncode,
        multi_write::MultiWrite, read_block::ReadBlock,
    },
};

pub use crate::operator::{
    column_split::ColumnarSplitMode, data_convert::DataConvertMode, delta_encode::DeltaEncodeMode,
};

#[derive(Copy, Clone)]
pub enum CompressionMode {
    None,
    Zstd,
}

pub enum Stage {
    DeltaEncode(DeltaEncodeMode),
    DataConvert(DataConvertMode),
    ColumnarSplit(ColumnarSplitMode),
}

pub struct Compressor {
    stages: Vec<Stage>,
    output: CompressionMode,
}

impl Compressor {
    #[must_use]
    pub fn new(stages: Vec<Stage>, output: CompressionMode) -> Self {
        Self { stages, output }
    }

    pub fn compress(
        &self,
        reader: &mut impl std::io::Read,
        mut fp: impl FnMut() -> std::io::Result<Box<dyn std::io::Write>>,
    ) -> std::io::Result<usize> {
        let mut out: Vec<Box<dyn std::io::Write>> = Vec::new();

        let mut n: Box<dyn Operator> = ReadBlock::new(reader, 128 * 1024);
        for s in &self.stages {
            match s {
                Stage::DeltaEncode(m) => n = DeltaEncode::new(n, *m),
                Stage::DataConvert(m) => n = DataConvert::new(n, *m),
                Stage::ColumnarSplit(m) => n = ColumnarSplit::new(n, *m),
            };
        }

        for _ in 0..n.num_output_buffers() {
            let f = fp()?;
            out.push(match self.output {
                CompressionMode::None => f,
                CompressionMode::Zstd => Box::new(zstd::Encoder::new(f, 9)?.auto_finish()),
            });
        }
        n = MultiWrite::new(
            n,
            out.iter_mut()
                .map(|o| o as &mut dyn std::io::Write)
                .collect(),
        );
        n.execute_discard()
    }
}
