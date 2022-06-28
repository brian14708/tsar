use crate::executor::{Buffer, Context, Operator};

pub(crate) struct MultiWrite<'p> {
    parent: Box<dyn Operator + 'p>,
    write: Vec<&'p mut dyn std::io::Write>,
}

impl<'p> MultiWrite<'p> {
    pub(crate) fn new(
        parent: Box<dyn Operator + 'p>,
        write: Vec<&'p mut dyn std::io::Write>,
    ) -> Box<Self> {
        assert_eq!(parent.num_output_buffers(), write.len());
        Box::new(Self { parent, write })
    }
}

impl Operator for MultiWrite<'_> {
    fn num_output_buffers(&self) -> usize {
        0
    }

    fn next(&mut self, ctx: &Context, _out: &mut [Buffer]) -> std::io::Result<usize> {
        let mut out = ctx.allocate(self.parent.num_output_buffers());
        let n = self.parent.next(ctx, &mut out)?;
        for i in 0..self.write.len() {
            _ = self.write[i].write(&out[i])?;
        }
        Ok(n)
    }
}
