#[allow(dead_code)]

use core::fmt::{self};
use core::str;


pub struct U8Writer<'a> {
    buf: &'a mut [u8],
    cursor: usize,
}

impl<'a> U8Writer<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        U8Writer { buf, cursor: 0 }
    }

    pub fn fill(&mut self, v: u8) {
      for i in self.cursor..self.buf.len() {
        self.buf[i] = v;
      }
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.buf[0..self.cursor]).unwrap()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
    }

    pub fn len(&self) -> usize {
        self.cursor
    }

    pub fn empty(&self) -> bool {
        self.cursor == 0
    }

    pub fn full(&self) -> bool {
        self.capacity() == self.cursor
    }
}

impl fmt::Write for U8Writer<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let cap = self.capacity();
        for (i, &b) in self.buf[self.cursor..cap]
            .iter_mut()
            .zip(s.as_bytes().iter())
        {
            *i = b;
        }
        self.cursor = usize::min(cap, self.cursor + s.as_bytes().len());
        Ok(())
    }
}

