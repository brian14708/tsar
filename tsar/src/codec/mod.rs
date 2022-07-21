use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;

mod compress;
mod convert;
mod split;
#[cfg(test)]
mod test_util;
mod zfp;

pub use compress::Compress;
pub use convert::Convert;
pub use split::Split;
pub use zfp::Zfp;

use crate::result::Result;

#[derive(Clone)]
pub struct BufferList {
    inner: SmallVec<[Vec<u8>; 4]>,
    n: usize,
}

impl BufferList {
    pub fn new() -> Self {
        Self {
            inner: SmallVec::new(),
            n: 0,
        }
    }

    pub fn reset(&mut self, n: usize) {
        if self.inner.len() < n {
            self.inner.resize_with(n, Vec::new);
        }
        self.n = n;
        self.inner.iter_mut().for_each(Vec::clear);
    }

    pub fn iter_mut(&mut self) -> BufferListIterMut {
        let n = self.n;
        BufferListIterMut {
            inner: self.inner.iter_mut(),
            n,
        }
    }

    pub fn iter(&self) -> BufferListIter {
        let n = self.n;
        BufferListIter {
            inner: self.inner.iter(),
            n,
        }
    }

    pub fn iter_slice(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.inner.iter().map(|f| f.as_slice())
    }
}

impl Default for BufferList {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BufferListIterMut<'a> {
    inner: core::slice::IterMut<'a, Vec<u8>>,
    n: usize,
}

impl<'a> Iterator for BufferListIterMut<'a> {
    type Item = &'a mut Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.inner.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}

impl ExactSizeIterator for BufferListIterMut<'_> {}

pub struct BufferListIter<'a> {
    inner: core::slice::Iter<'a, Vec<u8>>,
    n: usize,
}

impl<'a> Iterator for BufferListIter<'a> {
    type Item = &'a Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.inner.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}

impl ExactSizeIterator for BufferListIter<'_> {}

impl<'a> IntoIterator for &'a mut BufferList {
    type Item = &'a mut Vec<u8>;

    type IntoIter = BufferListIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a> IntoIterator for &'a BufferList {
    type Item = &'a Vec<u8>;

    type IntoIter = BufferListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Deref for BufferList {
    type Target = [Vec<u8>];

    fn deref(&self) -> &Self::Target {
        &self.inner.as_slice()[..self.n]
    }
}

impl DerefMut for BufferList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.as_mut_slice()[..self.n]
    }
}

pub trait Codec {
    fn encode<'a, I>(&self, data: I, out: &mut BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator;

    fn decode<'a, I>(&self, data: I, out: &mut BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator;
}
