use std::num::NonZeroUsize;

/// A data structure that uses a single, fixed-size buffer as if it were connected end-to-end
pub struct RingBuf {
    /// inner fixed-size memory block.
    inner_block: Vec<u8>,
    /// the read pointer offset.
    read_offset: usize,
    /// the write pointer offset.
    write_offset: usize,
}

impl RingBuf {
    /// Create a new `RingBuf` object with provided capacity.
    ///
    /// The provided `capacity` must be `NonZeroUsize`.
    pub fn with_capacity(value: NonZeroUsize) -> Self {
        Self {
            inner_block: vec![0; value.get()],
            read_offset: 0,
            write_offset: 0,
        }
    }

    /// Returns the number of bytes can be read.
    pub fn remaining(&self) -> usize {
        self.write_offset - self.read_offset
    }

    /// Returns the number of bytes that can be written from the current
    /// position until the end of the buffer is reached.
    pub fn remaining_mut(&self) -> usize {
        self.read_offset + self.inner_block.len() - self.write_offset
    }

    /// Returns a mutable slice starting at the `write pointer`,
    /// the length may shorter than [`remaining_mut`](Self::remaining_mut).
    pub fn chunk_mut(&mut self) -> &mut [u8] {
        let write_offset = self.write_offset % self.inner_block.len();

        let mut write_len = self.remaining_mut();

        if write_len > self.inner_block.len() - write_offset {
            write_len = self.inner_block.len() - write_offset;
        }

        &mut self.inner_block[write_offset..write_offset + write_len]
    }

    /// Returns a immutable slice starting at the `read pointer`,
    /// the length may shorter than [`remaining`](Self::remaining).
    pub fn chunk(&self) -> &[u8] {
        let read_offset = self.read_offset % self.inner_block.len();

        let mut read_len = self.remaining();

        if read_len > self.inner_block.len() - read_offset {
            read_len = self.inner_block.len() - read_offset;
        }

        &self.inner_block[read_offset..read_offset + read_len]
    }

    /// Advance the innner write cursor of this `RingBuf`.
    ///
    /// if the `offset` value > `remaining_mut` will cause an panic.
    pub fn advance_mut(&mut self, offset: usize) {
        if offset > self.remaining_mut() {
            panic!(
                "advance out of bounds: the remaining mut len is {} but advancing by {}",
                self.remaining_mut(),
                offset
            );
        }

        self.write_offset += offset;
    }

    /// Advance the innner read cursor of this `RingBuf`.
    ///
    /// if the `offset` value > `remaining` will cause an panic.
    pub fn advance(&mut self, offset: usize) {
        if offset > self.remaining() {
            panic!(
                "advance out of bounds: the remaining  len is {} but advancing by {}",
                self.remaining(),
                offset
            );
        }

        self.read_offset += offset;
    }

    /// Write data into ring buf.
    ///
    /// Returns the number of written bytes.
    /// the number of written bytes returned can be shorter than the length of the
    /// input buffer when the ring buf doesn’t have enough capacity.
    pub fn write(&mut self, mut buf: &[u8]) -> usize {
        let mut write_len = 0;

        while buf.len() > 0 && self.remaining_mut() > 0 {
            let chunk_mut = self.chunk_mut();

            if chunk_mut.len() >= buf.len() {
                chunk_mut[..buf.len()].copy_from_slice(buf);
                self.advance_mut(buf.len());
                write_len += buf.len();

                return write_len;
            }

            let advance = chunk_mut.len();

            chunk_mut.copy_from_slice(&buf[..advance]);

            write_len += advance;

            // move write cursor
            self.advance_mut(advance);

            // split source buf to `advance` offset.
            buf = &buf[advance..];
        }

        return write_len;
    }

    /// Read data from `RingBuf` and copy to provided mutable slice.
    ///
    /// Returns the number of read bytes.
    pub fn read(&mut self, mut buf: &mut [u8]) -> usize {
        let mut read_len = 0;

        while buf.len() > 0 && self.remaining() > 0 {
            let chunk = self.chunk();

            if chunk.len() >= buf.len() {
                buf.copy_from_slice(&chunk[..buf.len()]);

                self.advance(buf.len());
                read_len += buf.len();

                return read_len;
            }

            let advance = chunk.len();

            buf[..advance].copy_from_slice(chunk);

            read_len += advance;

            self.advance(advance);

            buf = &mut buf[advance..];
        }

        return read_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buf() {
        let capacity = 1024;

        let mut ring_buf = RingBuf::with_capacity(NonZeroUsize::new(1024).unwrap());

        assert_eq!(ring_buf.remaining(), 0);
        assert_eq!(ring_buf.remaining_mut(), capacity);

        // write data out of range.

        assert_eq!(ring_buf.write(&vec![1; capacity * 2]), ring_buf.remaining());

        assert_eq!(ring_buf.remaining(), capacity);
        assert_eq!(ring_buf.remaining_mut(), 0);

        let mut buf = vec![0; capacity / 2];

        assert_eq!(ring_buf.read(&mut buf), capacity / 2);

        assert_eq!(buf, vec![1; capacity / 2]);

        assert_eq!(ring_buf.write(&vec![2; capacity / 4]), capacity / 4);

        let mut buf = vec![0; capacity / 2];

        assert_eq!(ring_buf.read(&mut buf), capacity / 2);

        assert_eq!(buf, vec![1; capacity / 2]);

        let mut buf = vec![0; capacity / 8];

        assert_eq!(ring_buf.read(&mut buf), capacity / 8);

        assert_eq!(buf, vec![2; capacity / 8]);

        assert_eq!(ring_buf.remaining_mut(), 7 * capacity / 8);

        assert_eq!(ring_buf.write(&vec![3; capacity * 10]), 7 * capacity / 8);

        assert_eq!(ring_buf.remaining_mut(), 0);

        assert_eq!(ring_buf.remaining(), capacity);

        let mut buf = vec![0; capacity / 8];

        assert_eq!(ring_buf.read(&mut buf), capacity / 8);

        assert_eq!(buf, vec![2; capacity / 8]);

        assert_eq!(ring_buf.remaining_mut(), capacity / 8);

        assert_eq!(ring_buf.remaining(), 7 * capacity / 8);

        let mut buf = vec![0; capacity * 10];

        assert_eq!(ring_buf.read(&mut buf), 7 * capacity / 8);

        assert_eq!(buf[..7 * capacity / 8], vec![3; 7 * capacity / 8]);
    }
}
