use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;

pub struct Context {
    pool: object_pool::Pool<Vec<u8>>,
}

impl Context {
    fn new() -> Self {
        Self {
            pool: object_pool::Pool::new(4, || Vec::with_capacity(4096)),
        }
    }

    pub fn allocate(&self, cnt: usize) -> BufferList {
        BufferList::new(&self.pool, cnt, 4096)
    }
}

pub struct BufferList<'a> {
    pool: &'a object_pool::Pool<Vec<u8>>,
    inner: SmallVec<[Vec<u8>; 4]>,
}

impl<'a> BufferList<'a> {
    fn new(pool: &'a object_pool::Pool<Vec<u8>>, cnt: usize, cap: usize) -> Self {
        Self {
            pool,
            inner: (0..cnt)
                .map(|_| pool.pull(|| Vec::with_capacity(cap)))
                .map(|mut v| {
                    v.clear();
                    v.detach().1
                })
                .collect(),
        }
    }
}

impl Deref for BufferList<'_> {
    type Target = [Vec<u8>];
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl DerefMut for BufferList<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl Drop for BufferList<'_> {
    fn drop(&mut self) {
        std::mem::take(&mut self.inner)
            .into_iter()
            .for_each(|t| self.pool.attach(t))
    }
}

pub trait Operator {
    fn num_outputs(&self) -> usize;

    fn next(&mut self, ctx: &Context, out: &mut [Vec<u8>]) -> std::io::Result<usize>;
}

pub struct ExecReader<'o, Op>
where
    Op: Operator + ?Sized,
{
    op: &'o mut Op,
    ctx: Context,
    buf: Vec<u8>,
}

impl<Op> std::io::Read for ExecReader<'_, Op>
where
    Op: Operator + ?Sized,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while self.buf.is_empty() {
            let mut m = self.ctx.allocate(1);
            if self.op.next(&self.ctx, &mut m)? == 0 {
                return Ok(0);
            }
            std::mem::swap(&mut self.buf, &mut m[0]);
        }

        if self.buf.len() > buf.len() {
            buf.copy_from_slice(&self.buf[..buf.len()]);
            self.buf.truncate(buf.len());
            Ok(buf.len())
        } else {
            let r = self.buf.len();
            buf[..self.buf.len()].copy_from_slice(&self.buf);
            self.buf.clear();
            Ok(r)
        }
    }
}

pub trait Executable<Op: Operator + ?Sized> {
    fn execute_reader(&mut self) -> ExecReader<Op>;
    fn execute_discard(&mut self) -> std::io::Result<usize>;
}

impl<Op: Operator + ?Sized> Executable<Op> for dyn AsMut<Op> {
    fn execute_reader(&mut self) -> ExecReader<Op> {
        self.as_mut().execute_reader()
    }
    fn execute_discard(&mut self) -> std::io::Result<usize> {
        self.as_mut().execute_discard()
    }
}

impl<Op: Operator + ?Sized> Executable<Op> for Op {
    fn execute_reader(&mut self) -> ExecReader<Op> {
        assert!(self.num_outputs() == 1);
        ExecReader {
            op: self,
            ctx: Context::new(),
            buf: Vec::new(),
        }
    }

    fn execute_discard(&mut self) -> std::io::Result<usize> {
        let ctx = Context::new();
        let mut total = 0;
        let mut out_buffer = ctx.allocate(self.num_outputs());
        loop {
            match self.next(&ctx, &mut out_buffer)? {
                0 => break,
                v => total += v,
            }
            out_buffer.iter_mut().for_each(|v| v.clear());
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    struct Generator {
        cnt: u8,
    }

    impl Operator for Generator {
        fn num_outputs(&self) -> usize {
            1
        }

        fn next(&mut self, _ctx: &Context, out: &mut [Vec<u8>]) -> std::io::Result<usize> {
            out[0].push(self.cnt);
            self.cnt -= 1;
            if self.cnt == 0 {
                Ok(0)
            } else {
                Ok(1)
            }
        }
    }

    #[test]
    fn execute() {
        let mut g = Generator { cnt: 64 };
        let mut buf = vec![];
        g.execute_reader().read_to_end(&mut buf).unwrap();
        assert_eq!(buf.len(), 63);
    }
}
