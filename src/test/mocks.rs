use crate::connection::ReadWrite;
use std::io::{Read, Write};

pub struct MockReadWrite {
    pub(crate) read_buf: Vec<u8>,
    pub(crate) write_buf: Vec<u8>,
}

impl Read for MockReadWrite {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read_buf_slice = self.read_buf.as_slice();
        let res = read_buf_slice.read(buf);
        self.read_buf = read_buf_slice.to_vec();

        res
    }
}

impl Write for MockReadWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ReadWrite for MockReadWrite {
    fn as_read_mut(&mut self) -> &mut dyn Read {
        self
    }

    fn as_write_mut(&mut self) -> &mut dyn Write {
        self
    }
}
