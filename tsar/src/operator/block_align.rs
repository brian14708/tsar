use std::collections::VecDeque;

use crate::executor::Operator;

pub struct BlockAlign<'p> {
    parent: Box<dyn Operator + 'p>,
    params: Vec<usize>,
    buf: Vec<VecDeque<u8>>,
    eof: bool,
}

impl<'p> BlockAlign<'p> {
    pub fn new(parent: Box<dyn Operator + 'p>, v: impl IntoIterator<Item = usize>) -> Box<Self> {
        let params: Vec<_> = v.into_iter().collect();
        let buf = params.iter().map(|v| VecDeque::with_capacity(*v)).collect();
        Box::new(Self {
            parent,
            params,
            buf,
            eof: false,
        })
    }

    fn fill(&mut self, ctx: &crate::executor::Context) -> std::io::Result<usize> {
        let out = &mut ctx.allocate(self.parent.num_outputs());
        let n = self.parent.next(ctx, out)?;
        if n == 0 {
            self.eof = true;
            return Ok(0);
        }
        for (i, o) in out.iter_mut().enumerate() {
            self.buf[i].extend(o.iter());
        }
        Ok(n)
    }
}

impl Operator for BlockAlign<'_> {
    fn num_outputs(&self) -> usize {
        self.parent.num_outputs()
    }

    fn next(
        &mut self,
        ctx: &crate::executor::Context,
        out: &mut [Vec<u8>],
    ) -> std::io::Result<usize> {
        let mut sz = 0;
        for (i, o) in out.iter_mut().enumerate() {
            while !self.eof && self.buf[i].len() < self.params[i] {
                self.fill(ctx)?;
            }
            o.clear();
            let n = std::cmp::min(self.buf[i].len(), self.params[i]);
            o.extend(self.buf[i].iter().take(n));
            self.buf[i].drain(n..);
            sz += n;
        }
        Ok(sz)
    }
}
