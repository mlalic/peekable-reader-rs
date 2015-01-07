# peekable-reader-rs

[![Build Status](https://travis-ci.org/mlalic/peekable-reader-rs.svg?branch=master)](https://travis-ci.org/mlalic/peekable-reader-rs)

A utility struct for Rust: a wrapper around any `Reader`, which allows for performing `peek` operations, along with the usual `Reader` methods.

# Usage

For now, the `PeekableReader` exposes a single additional method: `fn peek_byte(&mut self) -> IoResult<u8>` which is analogous to the `read_byte` method exposed by the `Reader` trait, returning the result that the next `read_byte` will return. This allows the clients to check the result of the next read operation, without actually consuming the byte from the input.

The error semantics are such that any errors raised by the underlying wrapped `Reader` while performing a `peek` operation are *always* returned on a subsequent read method call, so as to perserve the invariant that the result of `peek_byte` is always the same as the result of the following `read_byte` operation.

Since the `PeekableReader` also implements the `Reader` trait, it is possible to seamlessly plug it in anywhere any other `Reader` implementation would fit.

```rust
// Wrap any `Reader` instance into a PeekableReader.
let mut peekable_reader = PeekableReader::new(MemReader::new(vec![1, 2, 3]));
// Now we can peek at the next byte, without consuming it...
let b = peekable_reader.peek_byte();
// Multiple calls of `peek_byte` before a read always return the same result
assert!(peekable_reader.peek_byte() == b);
// ...the result of the `peek_byte` is always exactly what the next
// `read_byte` call returns.
assert!(peekable_reader.read_byte() == b);
assert!(b.unwrap() == 1);

// A peeked byte is always correctly included in a read, regardless of which
// flavor of read is used from the `Reader` trait.
let _ = peekable_reader.peek_byte();
// (e.g. read into a buffer)
let mut buffer: Vec<u8> = vec![0, 0];
peekable_reader.read(buffer.as_mut_slice());
assert_eq!(vec![2, 3], buffer);

// Peeking at the end returns an EOF
assert!(match peekable_reader.peek_byte() {
    Err(IoError { kind: IoErrorKind::EndOfFile, .. }) => true,
    _ => false,
});
```

# License

[MIT](https://github.com/mlalic/peekable-reader-rs/blob/master/LICENSE)
