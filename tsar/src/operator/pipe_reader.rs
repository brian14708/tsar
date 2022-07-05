use std::{cell::RefCell, io::Read, rc::Rc};

use smallvec::SmallVec;

use crate::executor::{Context, Operator};

pub struct PipeReader<'p> {
    inner: Rc<RefCell<Inner<'p>>>,
    blk_size: usize,
    readers: Vec<Box<dyn std::io::Read + 'p>>,
}

#[derive(Clone)]
pub struct Adapter<'p> {
    inner: Rc<RefCell<Inner<'p>>>,
    idx: usize,
}

struct Inner<'p> {
    parent: Box<dyn Operator + 'p>,
    bufs: SmallVec<[Vec<u8>; 4]>,
    context: Option<*const Context>,
    eof: bool,
}

impl Read for Adapter<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut inner = self.inner.borrow_mut();
        while inner.bufs[self.idx].len() < buf.len() && !inner.eof {
            if inner.fill()? == 0 {
                inner.eof = true;
            }
        }

        let inner_buf = &mut inner.bufs[self.idx];
        let n = std::cmp::min(inner_buf.len(), buf.len());
        buf[..n].copy_from_slice(&inner_buf[..n]);
        inner_buf.drain(..n);
        Ok(n)
    }
}

impl Inner<'_> {
    fn fill(&mut self) -> std::io::Result<usize> {
        // SAFETY
        //
        //   Inner::Fill should only happen during Operator::next,
        //   context variable acts as a thread local variable.
        let ctx = unsafe {
            self.context
                .expect("PipeReader should only read on Operator::next()")
                .as_ref()
                .unwrap()
        };
        let mut tmp = ctx.allocate(self.bufs.len());
        _ = self.parent.next(ctx, &mut tmp)?;
        let mut sz = 0;
        self.bufs.iter_mut().zip(tmp.iter_mut()).for_each(|(a, b)| {
            sz += b.len();
            if a.is_empty() {
                std::mem::swap(a, b);
            } else {
                a.extend(b.iter());
            }
        });
        Ok(sz)
    }
}

impl<'p> PipeReader<'p> {
    pub fn new(
        parent: Box<dyn Operator + 'p>,
        blk_size: usize,
        mut r: impl FnMut(Adapter<'p>) -> Box<dyn std::io::Read + 'p>,
    ) -> Box<Self> {
        let n = parent.num_outputs();
        let inner = Rc::new(RefCell::new(Inner {
            parent,
            bufs: (0..n).map(|_| Vec::new()).collect(),
            context: None,
            eof: false,
        }));
        let mut readers = vec![];
        for idx in 0..n {
            readers.push(r(Adapter {
                inner: inner.clone(),
                idx,
            }))
        }
        Box::new(Self {
            inner,
            blk_size,
            readers,
        })
    }
}

impl Operator for PipeReader<'_> {
    fn num_outputs(&self) -> usize {
        self.inner.borrow().parent.num_outputs()
    }

    fn next(&mut self, ctx: &Context, out: &mut [Vec<u8>]) -> std::io::Result<usize> {
        self.inner.borrow_mut().context = Some(ctx);

        let mut sz = 0;
        let mut tmp = vec![0; self.blk_size];
        for (i, o) in out.iter_mut().enumerate() {
            let n = self.readers[i].read(&mut tmp)?;
            if n > 0 {
                if n > tmp.len() / 2 {
                    tmp.truncate(n);
                    std::mem::swap(&mut tmp, o);
                    tmp.resize(self.blk_size, 0);
                } else {
                    o.extend(&tmp[..n])
                }
                sz += n;
            }
        }

        self.inner.borrow_mut().context = None;
        Ok(sz)
    }
}
