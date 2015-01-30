use std::old_io::IoResult;

/// A wrapper around any struct implementing the `Reader` trait, additionally
/// allowing for `peek` operations to be performed. Therefore, the
/// `PeekableReader` struct also implements the `Reader` trait.
///
/// The primary invariant of the `PeekableReader` is that after calling the
/// `peek` method, the next `read_byte` call will return the same result as
/// the `peek` does. When the result is a byte (read off the wrapped reader),
/// any read-type method of the `Reader` trait will include the byte as the
/// first one. On the other hand, if the result is an `IoError`, the error
/// will be returned regardless of which read-type method of the `Reader` is
/// invoked. Consecutive `peek`s before any read-type operation is used
/// always return the same `IoResult`.
pub struct PeekableReader<R> {
    inner: R,
    peeked_result: Option<IoResult<u8>>,
}

impl<R: Reader> PeekableReader<R> {
    /// Initializes a new `PeekableReader` which wraps the given underlying
    /// reader.
    pub fn new(reader: R) -> PeekableReader<R> {
        PeekableReader {
            inner: reader,
            peeked_result: None,
        }
    }

    /// Returns the `IoResult` which the Reader will return on the next
    /// `get_byte` call.
    ///
    /// If the `IoResult` is indeed an `IoError`, the error will be returned
    /// for any subsequent read operation invoked upon the `Reader`.
    pub fn peek_byte(&mut self) -> IoResult<u8> {
        // Return either the currently cached peeked byte or obtain a new one
        // from the underlying reader.
        match self.peeked_result {
            Some(ref old_res) => old_res.clone(),
            None => {
                // First get the result of the read from the underlying reader
                self.peeked_result = Some(self.inner.read_byte());

                // Now just return that
                self.peeked_result.clone().unwrap()
            }
        }
    }
}

impl<R: Reader> Reader for PeekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }

        // First, put the byte that was read off the underlying reader in a
        // (possible) previous peek operation (if such a byte is indeed cached)
        let offset = match self.peeked_result.clone() {
            Some(Err(e)) => {
                self.peeked_result = None;
                return Err(e);
            },
            Some(Ok(b)) => {
                buf[0] = b;
                self.peeked_result = None;
                1
            },
            None => 0,
        };

        if offset == 1 && buf.len() == 1 {
            // We've filled the buffer by using the previously peeked byte
            Ok(1)
        } else {
            // We are still missing more bytes, so we read them from the
            // underlying reader and place them directly in the correct place
            // in the buffer.
            Ok(try!(self.inner.read(&mut buf[offset..])) + offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PeekableReader;
    use std::old_io::{MemReader, IoError, IoErrorKind, IoResult};

    /// A mock implementation of the `Reader` trait that returns a given
    /// IoError after a certain number of bytes has been read. The error is
    /// returned only once, with subsequent read calls completing successfully
    /// again.
    /// When the Reader successfully reads a byte, it is always `0u8`.
    struct MockReaderWithError {
        error: IoError,
        after: usize,
        bytes_read: usize,
        errored: bool,
    }

    impl MockReaderWithError {
        fn new(error: IoError, after: usize) -> MockReaderWithError {
            MockReaderWithError {
                error: error,
                after: after,
                bytes_read: 0,
                errored: false,
            }
        }
    }

    impl Reader for MockReaderWithError {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            self.bytes_read += buf.len();
            if self.bytes_read <= self.after || self.errored {
                for b in buf.iter_mut() {
                    *b = 0;
                }

                Ok(buf.len())
            } else {
                self.errored = true;
                Err(self.error.clone())
            }
        }
    }

    /// Helper function which checks whether the given result indicated EOF.
    fn is_eof<T>(res: IoResult<T>) -> bool {
        match res {
            Err(IoError {
                kind: IoErrorKind::EndOfFile,
                ..
            }) => true,
            _ => false,
        }
    }

    /// Helper function which just checks whether the given IoResult is equal
    /// to the given IoError.
    fn is_err<T>(err: &IoError, res: IoResult<T>) -> bool {
        match res {
            Err(ref result_error) => result_error == err,
            _ => false,
        }
    }

    /// Tests that it is possible to invoke the `peek_byte` method multiple
    /// times with the same effect, when there are no intermediate reads.
    #[test]
    fn multiple_peeks_idempotent() {
        let b = 1;
        let mut reader = PeekableReader::new(MemReader::new(vec!(b, b + 1)));

        // The first peek is correct?
        assert_eq!(b, reader.peek_byte().unwrap());
        // The second peek (with no reads in between) still returns the same
        // byte?
        assert_eq!(b, reader.peek_byte().unwrap());
    }

    /// Tests that the `peek_byte` method correctly returns an IoError
    /// returned by the underlying reader on consecutive peek_byte calls.
    #[test]
    fn peek_result_error() {
        let err = IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "Dummy error",
            detail: None,
        };
        let mut reader = PeekableReader::new(
            MockReaderWithError::new(err.clone(), 0));

        assert!(is_err(&err, reader.peek_byte()),
                "Expected an IoError!");
        assert!(is_err(&err, reader.peek_byte()),
                "Expected an IoError on the second peek too!");
    }

    /// Tests that performing multiple `peek_byte` calls after the `Reader`
    /// has reached EOF always returns EOF.
    #[test]
    fn multiple_peeks_at_eof() {
        let mut reader = PeekableReader::new(MemReader::new(vec!(1)));
        // Read/skip the first byte
        let _ = reader.read_byte();
        // Consume the EOF (and perform a sanity check that we have reached it)
        assert!(is_eof(reader.read_byte()));

        // All subsequent peeks return EOF
        assert!(is_eof(reader.peek_byte()));
        assert!(is_eof(reader.peek_byte()));
    }

    /// Tests that peeks and reads correctly work when the underlying reader
    /// only has a single byte.
    #[test]
    fn single_byte_reader_peek() {
        let b = 5u8;
        let mut reader = PeekableReader::new(MemReader::new(vec!(b)));

        // When we peek, we get the correct byte back?
        assert_eq!(b, reader.peek_byte().unwrap());
        // And now we also read it...
        assert_eq!(b, reader.read_byte().unwrap());
        // After this, both the peek and the read return an EOF?
        assert!(is_eof(reader.peek_byte()));
        assert!(is_eof(reader.read_byte()));
    }

    /// Tests that peeks and reads from an empty underlying reader instantly
    /// return EOF.
    #[test]
    fn empty_underlying_reader() {
        let mut reader = PeekableReader::new(MemReader::new(vec!()));

        // Must return EOF since the underlying reader is empty
        assert!(is_eof(reader.peek_byte()));
        assert!(is_eof(reader.read_byte()));
    }

    /// Tests that when we peek before every read, we correctly end up reading
    /// the entire contents of the underlying Reader.
    #[test]
    fn peek_read_alternate() {
        let vec: Vec<u8> = (0u8..5).collect();
        let mut reader = PeekableReader::new(MemReader::new(vec.clone()));

        for &b in vec.iter() {
            assert_eq!(b, reader.peek_byte().unwrap());
            assert_eq!(b, reader.read_byte().unwrap());
        }
        // Now we're at EOF
        assert!(is_eof(reader.peek_byte()));
        assert!(is_eof(reader.read_byte()));
    }

    /// Tests that a peek after the underlying reader has reached EOF fails
    /// with an appropriate error.
    #[test]
    fn peek_after_eof() {
        let mut reader = PeekableReader::new(MemReader::new(vec!(1)));
        // Read both the only byte from the reader and the EOF
        let _ = reader.read_byte();
        assert!(is_eof(reader.read_byte()));

        // Now the peek still correctly returns the EOF
        assert!(is_eof(reader.peek_byte()));
    }

    /// Tests that the PeekableReader behaves correctly when only Reader methods
    /// are used.
    #[test]
    fn sequential_reader() {
        let vec: Vec<u8> = (0u8..5).collect();
        let mut reader = PeekableReader::new(MemReader::new(vec.clone()));

        for &b in vec.iter() {
            assert_eq!(b, reader.read_byte().unwrap());
        }
        // Now we're at EOF
        assert!(is_eof(reader.read_byte()));
        // Still EOF...
        assert!(is_eof(reader.read_byte()));
    }

    /// Tests that a currently cached peeked byte does not get invalidated when
    /// the read method is invoked with an empty buffer.
    #[test]
    fn zero_length_buffer_read_does_not_invalidate_peek() {
        let vec: Vec<u8> = (0u8..5).collect();
        let mut reader = PeekableReader::new(MemReader::new(vec.clone()));
        let first_peek = reader.peek_byte().unwrap();

        let mut buffer = vec!();
        let _ = reader.read(buffer.as_mut_slice());

        assert_eq!(first_peek, reader.peek_byte().unwrap());
    }

    /// Tests that when the underlying `Reader` returns an IoError, the `peek`
    /// method keeps returning the same error for each call until the error is
    /// actually "read" by the client (by invoking one of the read methods of
    /// the `Reader`).
    #[test]
    fn always_returns_same_io_error_on_peek() {
        let err = IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "Dummy error",
            detail: None,
        };
        let mut reader = PeekableReader::new(
            MockReaderWithError::new(err.clone(), 1));
        // Read the first byte (no error)
        assert_eq!(0, reader.read_byte().unwrap());

        // This peek now signals an IoError
        assert!(is_err(&err, reader.peek_byte()),
                "Expected an IoError on the first peek");
        // The next peek still reports the same error...
        assert!(is_err(&err, reader.peek_byte()),
                "Expected an IoError on the second peek too");
        // The read also indicates the same error.
        assert!(is_err(&err, reader.read_byte()),
                "Expected an IoError on the read too");
        // The next peek has to retry the read -- which succeeds
        assert_eq!(0, reader.peek_byte().unwrap());
    }

    /// Test for the MockReaderWithError helper mock class...
    #[test]
    fn mock_sanity_check() {
        {
            let err = IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "Dummy error",
                detail: None,
            };
            let mut reader = MockReaderWithError::new(err.clone(), 1);

            // First byte is not a problem.
            assert_eq!(0, reader.read_byte().unwrap());
            // However, now the read fails...
            assert!(is_err(&err, reader.read_byte()),
                    "Expected an IoError on the read byte");
            // But, already on the next one, we're out of the woods.
            assert_eq!(0, reader.read_byte().unwrap());
        }
        {
            let err = IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "Dummy error",
                detail: None,
            };
            let mut reader = MockReaderWithError::new(err.clone(), 0);

            // The read fails on the first one...
            assert!(is_err(&err, reader.read_byte()),
                    "Expected an IoError on the read byte");
            // ...but now we're good.
            assert_eq!(0, reader.read_byte().unwrap());
        }
    }

}
