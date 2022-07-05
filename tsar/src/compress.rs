use crate::{
    executor::Executable,
    executor::Operator,
    operator::{
        column_split::ColumnarSplit,
        compress::{new_compress, new_decompress},
        data_convert::DataConvert,
        delta_encode::DeltaEncode,
        multi_write::MultiWrite,
        read_block::ReadBlock,
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
    Decompress(CompressMode),
}
pub struct Compressor {
    stages: Vec<Stage>,
}

impl Compressor {
    const BLK_SIZE: usize = 128 * 1024;

    #[must_use]
    pub fn new(stages: impl IntoIterator<Item = Stage>) -> Self {
        Self {
            stages: Vec::from_iter(stages),
        }
    }

    fn build_graph<'p>(
        stages: &Vec<Stage>,
        mut n: Box<dyn Operator + 'p>,
    ) -> Box<dyn Operator + 'p> {
        for s in stages {
            match s {
                Stage::DeltaEncode(m) => n = DeltaEncode::new(n, *m),
                Stage::DataConvert(m) => n = DataConvert::new(n, *m),
                Stage::ColumnarSplit(m) => n = ColumnarSplit::new(n, *m),
                Stage::Compress(m) => n = new_compress(n, *m),
                Stage::Decompress(m) => n = new_decompress(n, *m),
            };
        }
        n
    }

    pub fn compress_dryrun(
        &self,
        reader: &mut (impl std::io::Read + Clone),
    ) -> std::io::Result<(usize, f64)> {
        _ = Self::build_graph(&self.stages, ReadBlock::new(reader, Self::BLK_SIZE));
        Ok((0, 0.0))
    }

    pub fn compress(
        &self,
        reader: &mut impl std::io::Read,
        mut fp: impl FnMut() -> std::io::Result<Box<dyn std::io::Write>>,
    ) -> std::io::Result<usize> {
        let mut out: Vec<Box<dyn std::io::Write>> = Vec::new();

        let mut n: Box<dyn Operator> = ReadBlock::new(reader, Self::BLK_SIZE);
        n = Self::build_graph(&self.stages, n);

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
