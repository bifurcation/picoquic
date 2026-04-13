//! Bytestream utilities for reading and writing binary data.
//!
//! This module provides a cursor-like abstraction for serializing and
//! deserializing binary protocols, with support for QUIC variable-length
//! integers.
//!
//! Translated from picoquic/bytestream.c

use crate::intformat::{self, varint};

/// Maximum size for stack-allocated bytestream buffers.
pub const MAX_BUFFER_SIZE: usize = 2560;

/// Error type for bytestream operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamError;

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bytestream error: buffer overflow or underflow")
    }
}

impl std::error::Error for StreamError {}

/// A bytestream wraps a byte buffer with a read/write position.
///
/// This is similar to `std::io::Cursor` but designed for QUIC protocol
/// serialization with explicit error handling.
#[derive(Debug)]
pub struct ByteStream<'a> {
    data: &'a mut [u8],
    pos: usize,
}

/// A read-only bytestream for parsing.
#[derive(Debug)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

/// A bytestream with an owned buffer.
#[derive(Debug)]
pub struct ByteStreamBuf {
    buf: Vec<u8>,
    pos: usize,
}

impl<'a> ByteStream<'a> {
    /// Create a new bytestream wrapping a mutable buffer.
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the underlying data slice.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get a slice at the current position.
    pub fn ptr(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Get the total buffer size.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the current position (bytes written/read).
    pub fn length(&self) -> usize {
        self.pos
    }

    /// Get remaining bytes available.
    pub fn remain(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Reset position to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Clear buffer and reset position.
    pub fn clear(&mut self) {
        self.pos = 0;
        self.data.fill(0);
    }

    /// Check if the stream is at the end.
    pub fn finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skip forward by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), StreamError> {
        if self.remain() < n {
            self.pos = self.data.len();
            Err(StreamError)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    /// Write a u8.
    pub fn write_u8(&mut self, value: u8) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.data[self.pos] = value;
        self.pos += 1;
        Ok(())
    }

    /// Write a u16 in big-endian format.
    pub fn write_u16(&mut self, value: u16) -> Result<(), StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_16(&mut self.data[self.pos..], value);
        self.pos += 2;
        Ok(())
    }

    /// Write a u32 in big-endian format.
    pub fn write_u32(&mut self, value: u32) -> Result<(), StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_32(&mut self.data[self.pos..], value);
        self.pos += 4;
        Ok(())
    }

    /// Write a u64 in big-endian format.
    pub fn write_u64(&mut self, value: u64) -> Result<(), StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_64(&mut self.data[self.pos..], value);
        self.pos += 8;
        Ok(())
    }

    /// Write a QUIC variable-length integer.
    pub fn write_vint(&mut self, value: u64) -> Result<(), StreamError> {
        let len = varint::encode(&mut self.data[self.pos..], value);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(())
    }

    /// Write a byte buffer.
    pub fn write_buffer(&mut self, buffer: &[u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.data[self.pos..self.pos + buffer.len()].copy_from_slice(buffer);
        self.pos += buffer.len();
        Ok(())
    }

    /// Write a length-prefixed string (varint length + bytes).
    pub fn write_cstr(&mut self, s: &str) -> Result<(), StreamError> {
        self.write_vint(s.len() as u64)?;
        self.write_buffer(s.as_bytes())
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Peek a u8 without advancing position.
    pub fn peek_u8(&self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            return Err(StreamError);
        }
        Ok(self.data[self.pos])
    }

    /// Read a u16 in big-endian format.
    pub fn read_u16(&mut self) -> Result<u16, StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(value)
    }

    /// Read a u32 in big-endian format.
    pub fn read_u32(&mut self) -> Result<u32, StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(value)
    }

    /// Read a u64 in big-endian format.
    pub fn read_u64(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(value)
    }

    /// Read a QUIC variable-length integer.
    pub fn read_vint(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let (value, len) = varint::decode(&self.data[self.pos..]);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(value)
    }

    /// Skip a QUIC variable-length integer.
    pub fn skip_vint(&mut self) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let len = varint::decode_length(self.data[self.pos]);
        self.skip(len)
    }

    /// Read bytes into a buffer.
    pub fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        buffer.copy_from_slice(&self.data[self.pos..self.pos + buffer.len()]);
        self.pos += buffer.len();
        Ok(())
    }
}

impl<'a> ByteReader<'a> {
    /// Create a new read-only bytestream.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the underlying data slice.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get a slice at the current position.
    pub fn ptr(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Get the total buffer size.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Get remaining bytes available.
    pub fn remain(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Reset position to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Check if at end of stream.
    pub fn finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skip forward by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), StreamError> {
        if self.remain() < n {
            self.pos = self.data.len();
            Err(StreamError)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Peek a u8 without advancing.
    pub fn peek_u8(&self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            return Err(StreamError);
        }
        Ok(self.data[self.pos])
    }

    /// Read a u16 in big-endian format.
    pub fn read_u16(&mut self) -> Result<u16, StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(value)
    }

    /// Read a u32 in big-endian format.
    pub fn read_u32(&mut self) -> Result<u32, StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(value)
    }

    /// Read a u64 in big-endian format.
    pub fn read_u64(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(value)
    }

    /// Read a QUIC variable-length integer.
    pub fn read_vint(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let (value, len) = varint::decode(&self.data[self.pos..]);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(value)
    }

    /// Skip a QUIC variable-length integer.
    pub fn skip_vint(&mut self) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let len = varint::decode_length(self.data[self.pos]);
        self.skip(len)
    }

    /// Read bytes into a buffer.
    pub fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        buffer.copy_from_slice(&self.data[self.pos..self.pos + buffer.len()]);
        self.pos += buffer.len();
        Ok(())
    }

    /// Read a length-prefixed string.
    pub fn read_cstr(&mut self, max_len: usize) -> Result<String, StreamError> {
        let len = self.read_vint()? as usize;
        if len > max_len {
            return Err(StreamError);
        }
        let mut buf = vec![0u8; len];
        self.read_buffer(&mut buf)?;
        String::from_utf8(buf).map_err(|_| StreamError)
    }

    /// Skip a length-prefixed string.
    pub fn skip_cstr(&mut self) -> Result<(), StreamError> {
        let len = self.read_vint()? as usize;
        self.skip(len)
    }
}

impl ByteStreamBuf {
    /// Create a new bytestream with owned buffer.
    pub fn new(size: usize) -> Option<Self> {
        if size > MAX_BUFFER_SIZE {
            return None;
        }
        Some(Self {
            buf: vec![0u8; size],
            pos: 0,
        })
    }

    /// Get the underlying buffer.
    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    /// Get written data (up to current position).
    pub fn written(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    /// Get a mutable bytestream view.
    pub fn as_stream(&mut self) -> ByteStream<'_> {
        ByteStream {
            data: &mut self.buf,
            pos: self.pos,
        }
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Set the position (used after writing via as_stream).
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
}

/// Get the encoded length of a QUIC varint.
#[inline]
pub fn vint_len(value: u64) -> usize {
    varint::encode_length(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read_u8() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u8(0x42).is_ok());
        assert_eq!(stream.length(), 1);

        stream.reset();
        assert_eq!(stream.read_u8(), Ok(0x42));
    }

    #[test]
    fn test_write_read_u16() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u16(0x1234).is_ok());
        assert_eq!(stream.length(), 2);

        stream.reset();
        assert_eq!(stream.read_u16(), Ok(0x1234));
    }

    #[test]
    fn test_write_read_u32() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u32(0x12345678).is_ok());
        assert_eq!(stream.length(), 4);

        stream.reset();
        assert_eq!(stream.read_u32(), Ok(0x12345678));
    }

    #[test]
    fn test_write_read_u64() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u64(0x123456789ABCDEF0).is_ok());
        assert_eq!(stream.length(), 8);

        stream.reset();
        assert_eq!(stream.read_u64(), Ok(0x123456789ABCDEF0));
    }

    #[test]
    fn test_write_read_vint() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);

        // 1-byte varint
        assert!(stream.write_vint(37).is_ok());
        assert_eq!(stream.length(), 1);

        // 2-byte varint
        assert!(stream.write_vint(15293).is_ok());
        assert_eq!(stream.length(), 3);

        stream.reset();
        assert_eq!(stream.read_vint(), Ok(37));
        assert_eq!(stream.read_vint(), Ok(15293));
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buf = [0u8; 2];
        let mut stream = ByteStream::new(&mut buf);

        assert!(stream.write_u32(0x12345678).is_err());
        assert!(stream.finished());
    }

    #[test]
    fn test_skip() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        stream.write_u8(1).unwrap();
        stream.write_u8(2).unwrap();
        stream.write_u8(3).unwrap();

        stream.reset();
        assert!(stream.skip(2).is_ok());
        assert_eq!(stream.read_u8(), Ok(3));
    }

    #[test]
    fn test_skip_overflow() {
        let mut buf = [0u8; 5];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.skip(10).is_err());
        assert!(stream.finished());
    }

    #[test]
    fn test_write_read_buffer() {
        let mut buf = [0u8; 20];
        let mut stream = ByteStream::new(&mut buf);

        let data = b"hello";
        assert!(stream.write_buffer(data).is_ok());
        assert_eq!(stream.length(), 5);

        stream.reset();
        let mut out = [0u8; 5];
        assert!(stream.read_buffer(&mut out).is_ok());
        assert_eq!(&out, b"hello");
    }

    #[test]
    fn test_peek_u8() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        stream.write_u8(0x42).unwrap();

        stream.reset();
        assert_eq!(stream.peek_u8(), Ok(0x42));
        assert_eq!(stream.position(), 0); // position unchanged
        assert_eq!(stream.read_u8(), Ok(0x42));
        assert_eq!(stream.position(), 1); // position advanced
    }

    #[test]
    fn test_bytereader() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut reader = ByteReader::new(&data);

        assert_eq!(reader.read_u16(), Ok(0x1234));
        assert_eq!(reader.read_u16(), Ok(0x5678));
        assert!(reader.finished());
    }

    #[test]
    fn test_bytereader_cstr() {
        // varint length (5) followed by "hello"
        let data = [0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut reader = ByteReader::new(&data);

        assert_eq!(reader.read_cstr(100), Ok("hello".to_string()));
    }

    #[test]
    fn test_bytestream_buf() {
        let mut buf = ByteStreamBuf::new(100).unwrap();
        let len = {
            let mut stream = buf.as_stream();
            stream.write_u32(0xDEADBEEF).unwrap();
            stream.length()
        };
        buf.set_position(len);
        assert_eq!(buf.position(), 4);
        assert_eq!(&buf.written()[..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_bytestream_buf_too_large() {
        assert!(ByteStreamBuf::new(MAX_BUFFER_SIZE + 1).is_none());
    }

    #[test]
    fn test_remain() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert_eq!(stream.remain(), 10);
        stream.write_u32(0).unwrap();
        assert_eq!(stream.remain(), 6);
    }

    impl ByteStream<'_> {
        fn position(&self) -> usize {
            self.pos
        }
    }
}
