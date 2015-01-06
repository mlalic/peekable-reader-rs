use std::io::IoResult;

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

    pub fn peek(&mut self) -> IoResult<u8> {
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
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
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
            let (_, leftover) = buf.split_at_mut(offset);
            Ok(try!(self.inner.read(leftover)) + offset)
        }
    }
}

mod tests {
    use super::PeekableReader;
    use std::io::{MemReader, IoError, IoErrorKind, IoResult};

    /// A mock implementation of the `Reader` trait that returns a given
    /// IoError after a certain number of bytes has been read. The error is
    /// returned only once, with subsequent read calls completing successfully
    /// again.
    /// When the Reader successfully reads a byte, it is always `0u8`.
    struct MockReaderWithError {
        error: IoError,
        after: uint,
        bytes_read: uint,
        errored: bool,
    }

    impl MockReaderWithError {
        fn new(error: IoError, after: uint) -> MockReaderWithError {
            MockReaderWithError {
                error: error,
                after: after,
                bytes_read: 0,
                errored: false,
            }
        }
    }

    impl Reader for MockReaderWithError {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
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

    /// Tests that it is possible to invoke the `peek` method multiple times
    /// with the same effect, when there are no intermediate reads.
    #[test]
    fn multiple_peeks_idempotent() {
        let b = 1;
        let mut reader = PeekableReader::new(MemReader::new(vec!(b, b + 1)));

        // The first peek is correct?
        assert_eq!(b, reader.peek().unwrap());
        // The second peek (with no reads in between) still returns the same
        // byte?
        assert_eq!(b, reader.peek().unwrap());
    }

    /// Tests that the `peek` method correctly returns an IoError returned by
    /// the underlying reader on consecutive peek calls.
    #[test]
    fn peek_result_error() {
        let err = IoError {
            kind: IoErrorKind::OtherIoError,
            desc: "Dummy error",
            detail: None,
        };
        let mut reader = PeekableReader::new(
            MockReaderWithError::new(err.clone(), 0));

        assert!(is_err(&err, reader.peek()),
                "Expected an IoError!");
        assert!(is_err(&err, reader.peek()),
                "Expected an IoError on the second peek too!");
    }
    #[test]
    fn single_byte_reader_peek() {
        let b = 5u8;
        let mut reader = PeekableReader::new(MemReader::new(vec!(b)));

        // When we peek, we get the correct byte back?
        assert_eq!(b, reader.peek().unwrap());
    }

    #[test]
    fn empty_underlying_reader() {
        let mut reader = PeekableReader::new(MemReader::new(vec!()));

        // Must return EOF since the underlying reader is empty
        assert!(is_eof(reader.peek()));
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
