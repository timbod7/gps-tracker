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

    pub fn write_char(&mut self, c: char) -> Result<(), ()> {
        let mut buf: [u8; 4] = [0; 4];
        for cb in c.encode_utf8(&mut buf[..]).as_bytes().iter() {
            if self.full() {
                return Err(());
            }
            self.buf[self.cursor] = *cb;
            self.cursor += 1;
        }
        Ok(())
    }

    pub fn write_number(&mut self, width: usize, pad: char, value: u32) -> Result<(), ()> {
        let ndigits = {
            let mut ndigits = 0;
            let mut v = value;
            while v > 0 {
                v = v / 10;
                ndigits += 1;
            }
            ndigits
        };

        // Insert padding
        for _i in 0..width - ndigits {
            self.write_char(pad)?;
        }

        // Insert digits
        let v = value;
        for i in (0..ndigits).rev() {
            let f = 10u32.pow(i as u32);
            let c = char::from_digit((v / f) % 10, 10).unwrap();
            self.write_char(c)?;
        }

        Ok(())
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
