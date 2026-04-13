// Copyright (c) 2024 Patrick Elsen
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! Pure-Rust replacement for libevent's `evbuffer`.
//!
//! An `Evbuffer` is a growable byte buffer used for buffered I/O throughout
//! tmux. It supports appending data, draining consumed bytes from the front,
//! reading lines, and direct fd I/O.
//!
//! The `Evbuffer` struct provides the Rust-native API. Thin C-shaped wrapper
//! functions in `event_.rs` delegate to these methods so existing call sites
//! compile unchanged during the migration.

use std::ffi::c_int;

/// A growable byte buffer for buffered I/O.
///
/// Data is appended at the end and consumed (drained) from the front.
/// Internally uses a `Vec<u8>` with a read cursor to avoid copying on
/// every drain. The buffer is compacted when the cursor passes the halfway
/// point to bound wasted space.
pub struct Evbuffer {
    /// Backing storage.
    buf: Vec<u8>,
    /// Read cursor — bytes before this index have been logically consumed.
    cursor: usize,
}

impl Evbuffer {
    /// Create a new, empty buffer.
    pub fn new() -> Self {
        Evbuffer {
            buf: Vec::new(),
            cursor: 0,
        }
    }

    /// Number of readable bytes in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len() - self.cursor
    }

    /// Whether the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a slice of all readable bytes.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.cursor..]
    }

    /// Returns a mutable pointer to the start of readable data.
    ///
    /// # Safety
    /// The caller must not read beyond `self.len()` bytes.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.buf.as_mut_ptr().add(self.cursor) }
    }

    /// Append `data` to the end of the buffer.
    pub fn add(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Append the formatted string to the end of the buffer.
    pub fn add_printf(&mut self, args: std::fmt::Arguments) {
        use std::fmt::Write;
        struct VecWriter<'a>(&'a mut Vec<u8>);
        impl std::fmt::Write for VecWriter<'_> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0.extend_from_slice(s.as_bytes());
                Ok(())
            }
        }
        let _ = VecWriter(&mut self.buf).write_fmt(args);
    }

    /// Remove `n` bytes from the front of the buffer.
    ///
    /// If `n` exceeds the readable length, all data is drained.
    pub fn drain(&mut self, n: usize) {
        let n = n.min(self.len());
        self.cursor += n;
        self.maybe_compact();
    }

    /// Read and remove one line from the buffer, using LF as the line terminator.
    ///
    /// Returns `None` if no complete line is available. The returned bytes
    /// do NOT include the trailing newline.
    pub fn readln_lf(&mut self) -> Option<Vec<u8>> {
        let data = self.as_slice();
        let pos = memchr::memchr(b'\n', data)?;
        let line = data[..pos].to_vec();
        self.drain(pos + 1);
        Some(line)
    }

    /// Read from a file descriptor into the buffer.
    ///
    /// `howmuch` is a hint for how many bytes to try to read. If negative,
    /// uses a default of 4096.
    ///
    /// Returns the number of bytes read, 0 on EOF, or -1 on error.
    pub fn read_from_fd(&mut self, fd: c_int, howmuch: i32) -> i32 {
        let howmuch = if howmuch < 0 { 4096 } else { howmuch as usize };

        self.buf.reserve(howmuch);
        let start = self.buf.len();
        unsafe {
            let ptr = self.buf.as_mut_ptr().add(start);
            let n = libc::read(fd, ptr.cast(), howmuch);
            if n > 0 {
                self.buf.set_len(start + n as usize);
            }
            n as i32
        }
    }

    /// Write data from the buffer to a file descriptor.
    ///
    /// Returns the number of bytes written, or -1 on error.
    /// Written bytes are automatically drained.
    pub fn write_to_fd(&mut self, fd: c_int) -> i32 {
        if self.is_empty() {
            return 0;
        }
        let data = self.as_slice();
        let n = unsafe { libc::write(fd, data.as_ptr().cast(), data.len()) };
        if n > 0 {
            self.drain(n as usize);
        }
        n as i32
    }

    /// Compact internal storage if the cursor has passed the halfway point.
    fn maybe_compact(&mut self) {
        if self.cursor > 0 && (self.cursor >= self.buf.len() / 2 || self.is_empty()) {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }
    }
}

impl Default for Evbuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf = Evbuffer::new();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert_eq!(buf.as_slice(), &[]);
    }

    #[test]
    fn add_and_len() {
        let mut buf = Evbuffer::new();
        buf.add(b"hello");
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_slice(), b"hello");

        buf.add(b" world");
        assert_eq!(buf.len(), 11);
        assert_eq!(buf.as_slice(), b"hello world");
    }

    #[test]
    fn drain_partial() {
        let mut buf = Evbuffer::new();
        buf.add(b"hello world");
        buf.drain(6);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_slice(), b"world");
    }

    #[test]
    fn drain_all() {
        let mut buf = Evbuffer::new();
        buf.add(b"hello");
        buf.drain(5);
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_more_than_available() {
        let mut buf = Evbuffer::new();
        buf.add(b"hi");
        buf.drain(100);
        assert!(buf.is_empty());
    }

    #[test]
    fn readln_lf_single_line() {
        let mut buf = Evbuffer::new();
        buf.add(b"hello\n");
        let line = buf.readln_lf().unwrap();
        assert_eq!(line, b"hello");
        assert!(buf.is_empty());
    }

    #[test]
    fn readln_lf_no_newline() {
        let mut buf = Evbuffer::new();
        buf.add(b"hello");
        assert!(buf.readln_lf().is_none());
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn readln_lf_multiple_lines() {
        let mut buf = Evbuffer::new();
        buf.add(b"line1\nline2\nline3\n");

        assert_eq!(buf.readln_lf().unwrap(), b"line1");
        assert_eq!(buf.readln_lf().unwrap(), b"line2");
        assert_eq!(buf.readln_lf().unwrap(), b"line3");
        assert!(buf.readln_lf().is_none());
        assert!(buf.is_empty());
    }

    #[test]
    fn readln_lf_empty_line() {
        let mut buf = Evbuffer::new();
        buf.add(b"\n");
        let line = buf.readln_lf().unwrap();
        assert_eq!(line, b"");
        assert!(buf.is_empty());
    }

    #[test]
    fn readln_lf_partial_then_complete() {
        let mut buf = Evbuffer::new();
        buf.add(b"hel");
        assert!(buf.readln_lf().is_none());
        buf.add(b"lo\n");
        let line = buf.readln_lf().unwrap();
        assert_eq!(line, b"hello");
    }

    #[test]
    fn add_printf() {
        let mut buf = Evbuffer::new();
        buf.add_printf(format_args!("hello {} {}", "world", 42));
        assert_eq!(buf.as_slice(), b"hello world 42");
    }

    #[test]
    fn compaction_after_drain() {
        let mut buf = Evbuffer::new();
        buf.add(&[b'x'; 100]);
        buf.drain(80);
        assert_eq!(buf.len(), 20);
        assert_eq!(buf.as_slice(), &[b'x'; 20]);
        // Cursor should have been reset by compaction.
        assert_eq!(buf.cursor, 0);
    }

    #[test]
    fn interleaved_add_drain() {
        let mut buf = Evbuffer::new();
        for i in 0..100u8 {
            buf.add(&[i; 10]);
            buf.drain(5);
        }
        assert_eq!(buf.len(), 500);
    }

    #[test]
    fn fd_round_trip() {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);

        let mut write_buf = Evbuffer::new();
        write_buf.add(b"hello from pipe");

        let written = write_buf.write_to_fd(fds[1]);
        assert_eq!(written, 15);
        assert!(write_buf.is_empty());

        let mut read_buf = Evbuffer::new();
        let nread = read_buf.read_from_fd(fds[0], -1);
        assert_eq!(nread, 15);
        assert_eq!(read_buf.as_slice(), b"hello from pipe");

        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    #[test]
    fn readln_crlf_not_supported_uses_lf() {
        // Even if data has \r\n, readln_lf splits on \n only.
        // The \r is part of the returned line data.
        let mut buf = Evbuffer::new();
        buf.add(b"hello\r\nworld\n");
        let line = buf.readln_lf().unwrap();
        assert_eq!(line, b"hello\r");
        let line = buf.readln_lf().unwrap();
        assert_eq!(line, b"world");
    }

    #[test]
    fn as_mut_ptr_and_len() {
        let mut buf = Evbuffer::new();
        buf.add(b"test");
        let ptr = buf.as_mut_ptr();
        let len = buf.len();
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert_eq!(slice, b"test");
    }
}
