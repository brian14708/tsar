use crate::executor::{Buffer, Context, Operator};

pub struct MultiWrite<'p> {
    parent: Box<dyn Operator + 'p>,
    write: Vec<&'p mut dyn std::io::Write>,
}

impl<'p> MultiWrite<'p> {
    pub fn new(
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
        let _ = self.parent.next(ctx, &mut out)?;
        let mut sz = 0;
        for i in 0..self.write.len() {
            sz += out[i].len();
            self.write[i].write_all(&out[i])?;
        }
        Ok(sz)
    }
}
