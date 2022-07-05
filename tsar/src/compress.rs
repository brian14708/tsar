use crate::{
    executor::Executable,
    executor::Operator,
    operator::{
        column_split::ColumnarSplit, compress::Compress, data_convert::DataConvert,
        delta_encode::DeltaEncode, multi_write::MultiWrite, pipe::Pipe, read_block::ReadBlock,
    },
};

pub use crate::operator::{
    column_split::ColumnarSplitMode, compress::CompressMode, data_convert::DataConvertMode,
    delta_encode::DeltaEncodeMode,
};

pub enum Stage {
    DeltaEncode(DeltaEncodeMode),
    DataConvert(DataConvertMode),
    ColumnarSplit(ColumnarSplitMode),
    Compress(CompressMode),
}
pub struct Compressor {
    stages: Vec<Stage>,
}

impl Compressor {
    #[must_use]
    pub fn new(stages: impl IntoIterator<Item = Stage>) -> Self {
        Self {
            stages: Vec::from_iter(stages),
        }
    }

    pub fn compress_dryrun(
        &self,
        reader: &mut (impl std::io::Read + Clone),
    ) -> std::io::Result<(usize, f64)> {
        let mut n: Box<dyn Operator> = ReadBlock::new(reader, 128 * 1024);
        for s in &self.stages {
            match s {
                Stage::DeltaEncode(m) => n = DeltaEncode::new(n, *m),
                Stage::DataConvert(m) => n = DataConvert::new(n, *m),
                Stage::ColumnarSplit(m) => n = ColumnarSplit::new(n, *m),
                Stage::Compress(m) => n = Compress::new(n, *m),
            };
        }

        Ok((0, 0.0))
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
                Stage::Compress(m) => match m {
                    CompressMode::Zstd(level) => {
                        n = Pipe::new(n, |f| {
                            Box::new(zstd::Encoder::new(f, *level).unwrap().auto_finish())
                        })
                    }
                },
            };
        }

        for _ in 0..n.num_outputs() {
            out.push(fp()?);
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
