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

mod tests {
    use super::PeekableReader;
    use std::io::{MemReader, IoError, IoErrorKind, IoResult};

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
}
