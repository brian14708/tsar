use std::path::Path;

use crate::{
    executor::Executable,
    executor::Operator,
    operator::{
        column_split::{ColumnarSplit, ColumnarSplitMode},
        data_convert::{DataConvert, DataConvertMode},
        delta_encode::{DeltaEncode, DeltaEncodeMode},
        multi_write::MultiWrite,
        read_block::ReadBlock,
    },
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
    pub fn new(stages: Vec<Stage>, output: CompressionMode) -> Self {
        Self { stages, output }
    }

    pub fn compress(
        &self,
        reader: &mut impl std::io::Read,
        p: impl AsRef<Path>,
    ) -> std::io::Result<usize> {
        let mut out: Vec<Box<dyn std::io::Write>> = vec![];

        let mut n: Box<dyn Operator> = ReadBlock::new(reader, 128 * 1024);
        for s in self.stages.iter() {
            match s {
                Stage::DeltaEncode(m) => n = DeltaEncode::new(n, *m),
                Stage::DataConvert(m) => n = DataConvert::new(n, *m),
                Stage::ColumnarSplit(m) => n = ColumnarSplit::new(n, *m),
            };
        }

        let opener = |p: &Path| -> std::io::Result<Box<dyn std::io::Write>> {
            let f = std::fs::File::create(p)?;
            Ok(match self.output {
                CompressionMode::None => Box::new(f),
                CompressionMode::Zstd => Box::new(zstd::Encoder::new(f, 9)?.auto_finish()),
            })
        };

        if n.num_output_buffers() == 1 {
            out.push(opener(p.as_ref())?);
        } else {
            for i in 0..n.num_output_buffers() {
                out.push(opener(
                    p.as_ref().with_extension(format!("{}", i)).as_path(),
                )?);
            }
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
