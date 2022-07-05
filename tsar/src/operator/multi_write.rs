use smallvec::SmallVec;

use crate::executor::{Context, Operator};

pub struct MultiWrite<'p> {
    parent: Box<dyn Operator + 'p>,
    write: SmallVec<[&'p mut dyn std::io::Write; 4]>,
}

impl<'p> MultiWrite<'p> {
    pub fn new(
        parent: Box<dyn Operator + 'p>,
        write: impl IntoIterator<Item = &'p mut dyn std::io::Write>,
    ) -> Box<Self> {
        let write: SmallVec<_> = write.into_iter().collect();
        assert_eq!(parent.num_outputs(), write.len());
        Box::new(Self { parent, write })
    }
}

impl Operator for MultiWrite<'_> {
    fn num_outputs(&self) -> usize {
        0
    }

    fn next(&mut self, ctx: &Context, _out: &mut [Vec<u8>]) -> std::io::Result<usize> {
        let mut out = ctx.allocate(self.parent.num_outputs());
        let _ = self.parent.next(ctx, &mut out)?;
        let mut sz = 0;
        for i in 0..self.write.len() {
            sz += out[i].len();
            self.write[i].write_all(&out[i])?;
        }
        Ok(sz)
    }
}
