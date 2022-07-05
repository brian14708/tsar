use crate::executor::Operator;

pub struct ByteCount<'p> {
    parent: Box<dyn Operator + 'p>,
    cnt: &'p mut usize,
}

impl<'p> ByteCount<'p> {
    pub fn new(parent: Box<dyn Operator + 'p>, cnt: &'p mut usize) -> Box<Self> {
        Box::new(Self { parent, cnt })
    }
}

impl Operator for ByteCount<'_> {
    fn num_outputs(&self) -> usize {
        self.parent.num_outputs()
    }

    fn next(
        &mut self,
        ctx: &crate::executor::Context,
        out: &mut [Vec<u8>],
    ) -> std::io::Result<usize> {
        let n = self.parent.next(ctx, out)?;
        for o in out {
            *self.cnt += o.len();
        }
        Ok(n)
    }
}
