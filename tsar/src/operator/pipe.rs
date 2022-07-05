use std::{cell::RefCell, io::Write, ops::DerefMut, rc::Rc};

use crate::executor::{Buffer, Context, Operator};

pub struct Pipe<'p> {
    parent: Box<dyn Operator + 'p>,
    buf: Vec<Adapter>,
    writers: Vec<Box<dyn std::io::Write + 'p>>,
    eof: bool,
}

#[derive(Clone)]
pub struct Adapter {
    buf: Rc<RefCell<Vec<u8>>>,
}

impl Adapter {
    fn new() -> Self {
        Self {
            buf: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn len(&self) -> usize {
        self.buf.borrow().len()
    }
}

impl Write for Adapter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buf.borrow_mut().flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.buf.borrow_mut().write_all(buf)
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> std::io::Result<()> {
        self.buf.borrow_mut().write_fmt(fmt)
    }
}

impl<'p> Pipe<'p> {
    pub fn new(
        parent: Box<dyn Operator + 'p>,
        mut w: impl FnMut(Adapter) -> Box<dyn std::io::Write + 'p>,
    ) -> Box<Self> {
        let buf = (0..parent.num_output_buffers())
            .map(|_| Adapter::new())
            .collect::<Vec<_>>();
        let writers = buf.iter().map(|f| w(f.clone())).collect();

        Box::new(Self {
            parent,
            buf,
            writers,
            eof: false,
        })
    }
}

impl Operator for Pipe<'_> {
    fn num_output_buffers(&self) -> usize {
        self.parent.num_output_buffers()
    }

    fn next(&mut self, ctx: &Context, out: &mut [Buffer]) -> std::io::Result<usize> {
        while !self.eof && self.buf.iter().all(|v| v.len() == 0) {
            if self.parent.next(ctx, out)? == 0 {
                self.eof = true;
                for w in self.writers.iter_mut() {
                    w.flush()?;
                }
                self.writers.clear();
                break;
            }
            for (i, s) in out.iter_mut().enumerate() {
                assert!(self.buf[i].len() == 0);
                if !s.is_empty() {
                    self.writers[i].write_all(s)?;
                    s.clear();
                }
            }
        }

        Ok(self
            .buf
            .iter_mut()
            .enumerate()
            .map(|(i, s)| match s.len() {
                0 => 0,
                l => {
                    std::mem::swap(out[i].deref_mut(), s.buf.borrow_mut().deref_mut());
                    assert!(s.len() == 0);
                    l
                }
            })
            .sum())
    }
}
