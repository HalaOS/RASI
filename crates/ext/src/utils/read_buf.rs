use bytes::{Buf, BufMut, Bytes, BytesMut};

/// A wrapper around a byte buffer that is incrementally filled and initialized.
///
/// This type is a sort of "double cursor". It tracks three regions in the
/// buffer: a region at the beginning of the buffer that has been logically
/// filled with data, a region that has been initialized at some point but not
/// yet logically filled, and a region at the end that may be uninitialized.
/// The filled region is guaranteed to be a subset of the initialized region.
///
/// In summary, the contents of the buffer can be visualized as:
///
/// ```not_rust
/// [             capacity              ]
/// [ filled |         unfilled         ]
/// [    initialized    | uninitialized ]
/// ```
///
/// It is undefined behavior to de-initialize any bytes from the uninitialized
/// region, since it is merely unknown whether this region is uninitialized or
/// not, and if part of it turns out to be initialized, it must stay initialized.
pub struct ReadBuf {
    inner: BytesMut,
}

impl ReadBuf {
    /// Creates a new `ReadBuf` with the specified capacity.
    ///
    /// The returned `ReadBuf` will be able to hold at least `capacity` bytes
    /// without reallocating.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: BytesMut::with_capacity(capacity),
        }
    }

    /// Returns a mutable slice starting at the current BufMut position and of
    /// length between 0 and [`ReadBuf::remaining_mut()`]. Note that this *can* be shorter than the
    /// whole remainder of the buffer (this allows non-continuous implementation).
    pub fn chunk_mut(&mut self) -> &mut [u8] {
        let dst = self.inner.chunk_mut();

        unsafe { &mut *(dst as *mut _ as *mut [u8]) }
    }

    /// Returns the number of bytes that can be written from the current
    /// position until the end of the buffer is reached.
    ///
    /// This value is greater than or equal to the length of the slice returned
    /// by `chunk_mut()`.
    pub fn remaining_mut(&self) -> usize {
        self.inner.remaining_mut()
    }

    /// Advance the internal cursor of the BufMut
    ///
    /// The next call to [`chunk_mut`](Self::chunk_mut) will return a slice starting `cnt` bytes
    /// further into the underlying buffer.
    pub fn advance_mut(&mut self, advance: usize) {
        unsafe {
            self.inner.advance_mut(advance);
        }
    }

    /// Consume [`ReadBuf`] and convert into [`BytesMut`]
    pub fn into_bytes_mut(mut self, advance: Option<usize>) -> BytesMut {
        if let Some(advance) = advance {
            self.advance_mut(advance);
        }

        self.inner
    }

    /// Consume [`ReadBuf`] and convert into [`Bytes`]
    pub fn into_bytes(self, advance: Option<usize>) -> Bytes {
        self.into_bytes_mut(advance).into()
    }

    /// Returns a slice starting at the current position and of length between 0
    /// and [`ReadBuf::remaining()`](Self::remaining). Note that this *can* return shorter slice (this allows
    /// non-continuous internal representation).
    pub fn chunk(&self) -> &[u8] {
        self.inner.chunk()
    }

    /// Returns the number of bytes between the current position and the end of
    /// the buffer.
    ///
    /// This value is greater than or equal to the length of the slice returned
    /// by [`chunk()`](Self::chunk).
    pub fn remaining(&self) -> usize {
        self.inner.remaining()
    }
}
