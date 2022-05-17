use crate::sys::syscall3;
use core::convert::From;

static mut STDIN: Option<InputBuffer<4096>> = None;
static mut STDOUT: Option<OutputBuffer<4>> = None;
static mut STDERR: Option<OutputBuffer<4096>> = None;

fn dbgput<T: AsRef<[u8]>>(ch: T) -> usize {
    let ch = ch.as_ref();
    unsafe { syscall3(4, 2, ch.as_ptr() as usize, ch.len()) }
}

pub(crate) trait Read {
    fn getc(&mut self) -> Option<u8>;
    fn ungetc(&mut self) -> Option<()>;
    fn peek(&mut self) -> Option<u8> {
        let answer = self.getc();
        self.ungetc();
        answer
    }
}

pub(crate) trait Write {
    fn putc(&mut self, ch: u8) -> usize;
    fn puts<T: AsRef<[u8]>>(&mut self, ch: T) -> usize;
}

pub(crate) struct Cursor<'a> {
    offset: usize,
    ptr: &'a [u8]
}

impl<'a, 'b: 'a> From<&'b str> for Cursor<'a> {
    fn from(input: &'a str) -> Self {
        Self {
            offset: 0,
            ptr: input.as_bytes()
        }
    }
}

impl<'a, 'b: 'a> From<&'b [u8]> for Cursor<'a> {
    fn from(ptr: &'a [u8]) -> Self {
        Self {
            offset: 0,
            ptr
        }
    }
}

impl<'a> Read for Cursor<'a> {
    fn getc(&mut self) -> Option<u8> {
        self.offset += 1;
        if self.offset == self.ptr.len() {
            None
        } else {
            Some(self.ptr[self.offset - 1])
        }
    }

    fn ungetc(&mut self) -> Option<()> {
        if self.offset == 0 {
            None
        } else {
            self.offset -= 1;
            Some(())
        }
    }
}

/// A wrapper around input. This is copied from a design presented
/// in "Compilers: Principles, Tools, and Techniques" (Aho 1984). This design
/// has two goals:
///
/// 1. Allow efficient single-character "peeking" of input -- that is, expose the
///    ability to pull one character out of the stream at a time AND rewind the stream
///    one character at a time.
/// 2. Minimize the number of system calls made to read data.
///
/// To that end, we allocate a buffer of N bytes, where N is a power of two. We divide that
/// in half, like so:
///
///    0                N/2                N
///    +-----------------+-----------------+
///    | side a          | side b          |
///    +-----------------+-----------------+
///
/// We fill one side of the whole buffer at a time. Whenever the program calls getc(), it advances
/// a cursor in the buffer. If the position of the cursor after the call is N/2 or N, we fill() the
/// next side of the buffer: "side b" for N/2 and "side a" for N.
struct InputBuffer<const N: usize> {
    buf: [u8; N],
    cursor: usize,
    eofidx: usize,
    fd: usize
}

impl<const N: usize> InputBuffer<N> {
    const MIDPOINT: usize = if N & (N - 1) != 0 {
        panic!("N must be a power of 2")
    } else if N == usize::max_value() {
        panic!("N must be < usize::max_value")
    } else {
        N >> 1
    };

    fn new(fd: usize) -> Self {
        let mut s = Self {
            fd,
            buf: [0u8; N],
            cursor: 0,
            eofidx: N + 1,
        };

        s.fill();
        s
    }

    fn fill(&mut self) {
        let mem = &mut self.buf[self.cursor..self.cursor + Self::MIDPOINT];
        let read_bytes = unsafe { syscall3(3, self.fd, mem.as_mut_ptr() as usize, Self::MIDPOINT) };
        if read_bytes < Self::MIDPOINT {
            self.eofidx = self.cursor + read_bytes + 1;
        }
    }
}

impl<const N: usize> Read for InputBuffer<N> {
    fn getc(&mut self) -> Option<u8> {
        let ch = self.buf[self.cursor];
        let last = self.cursor;
        self.cursor = (self.cursor + 1) & N - 1;

        if self.cursor == 0 || self.cursor == Self::MIDPOINT {
            self.fill();
        }

        // TODO: fuse on eof
        if last != self.eofidx {
            Some(ch)
        } else {
            None
        }
    }

    fn ungetc(&mut self) -> Option<()> {
        // TODO:
        // - handle "unget" across eof
        // - handle initial "unget"
        self.cursor = if self.cursor == 0 {
            N - 1
        } else {
            self.cursor - 1
        };

        Some(())
    }
}

struct OutputBuffer<const N: usize> {
    buf: [u8; N],

    /// The position for the next input character to be written at.
    next_write_idx: usize,

    /// The position of the last flushed character.
    last_flushed_idx: usize,

    /// A file descriptor.
    fd: usize
}

impl<const N: usize> OutputBuffer<N> {
    const MODULO_MASK: usize = if N & (N - 1) != 0 {
        panic!("N must be a power of 2")
    } else if N == usize::max_value() {
        panic!("N must be < usize::max_value")
    } else {
        N - 1
    };

    fn new(fd: usize) -> Self {
        Self {
            next_write_idx: 0,
            last_flushed_idx: Self::MODULO_MASK,
            buf: [0u8; N],
            fd
        }
    }

    fn flush_all(&mut self) -> Option<usize> {
        Some(self.flush().unwrap_or(0) + self.flush().unwrap_or(0))
    }

    fn flush(&mut self) -> Option<usize> {
        // +-------------------+------------------+--------------
        // | next_write_idx    | last_flushed_idx | action
        // +-------------------+------------------+--------------
        // | 0                 | 0                | INVALID; no action
        // | 0                 | N-1              | no action, nothing to write
        // | M                 | M-1              | no action, nothing to write
        // | 12                | 14               | write N-13 bytes starting at 15; update cursor to N-1
        // | 15                | 1                | write 14 bytes; update cursor to 14
        Some(match self.next_write_idx.cmp(&self.last_flushed_idx) {
            core::cmp::Ordering::Equal => {
                // technically, this should never happen.
                0
            },

            core::cmp::Ordering::Less => {
                if self.last_flushed_idx == Self::MODULO_MASK {
                    let slice = &self.buf[0..self.next_write_idx];
                    if slice.is_empty() {
                        return None
                    }

                    self.last_flushed_idx = (self.next_write_idx - 1) & Self::MODULO_MASK;

                    unsafe { syscall3(4, self.fd, slice.as_ptr() as usize, slice.len()) }
                } else {
                    let slice = &self.buf[(self.last_flushed_idx + 1)..];
                    if slice.is_empty() {
                        return None
                    }

                    self.last_flushed_idx = Self::MODULO_MASK;

                    unsafe { syscall3(4, self.fd, slice.as_ptr() as usize, slice.len()) }
                }
            }

            core::cmp::Ordering::Greater => {
                let slice = &self.buf[self.last_flushed_idx + 1..self.next_write_idx];
                if slice.is_empty() {
                    return None
                }

                self.last_flushed_idx = self.next_write_idx - 1;

                unsafe { syscall3(4, self.fd, slice.as_ptr() as usize, slice.len()) }
            }
        })
    }
}

impl<const N: usize> Write for OutputBuffer<N> {
    fn putc(&mut self, ch: u8) -> usize {
        self.puts([ch])
    }

    fn puts<T: AsRef<[u8]>>(&mut self, bytes: T) -> usize {
        let bytes = bytes.as_ref();
        // if we would place next_write_idx after the current last_flushed_idx, we have to
        // split our write. Write until next_write_idx _would be_ last_flushed_idx - 1; then
        // flush.
        //
        // it might be faster to split this into two loops with a flush in-between, but my brain is
        // not working today. at the very least this is the generic cast so we can handle writes
        // >N.
        let start = self.next_write_idx;
        let mut flush_idx = self.last_flushed_idx.checked_sub(1).unwrap_or(Self::MODULO_MASK);
        for (offset, byte) in bytes.iter().enumerate() {
            let idx = (start + offset) & Self::MODULO_MASK;
            if idx == flush_idx {
                self.next_write_idx = idx;
                self.flush();
                flush_idx = self.last_flushed_idx.checked_sub(1).unwrap_or(Self::MODULO_MASK);
            }
            self.buf[idx] = *byte;
        }

        self.next_write_idx = (start + bytes.len()) & Self::MODULO_MASK;

        bytes.len()
    }
}

pub(crate) fn putc(ch: char) -> usize {
    puts([ch as u8])
}

pub(crate) fn puts<T: AsRef<[u8]>>(ch: T) -> usize {
    let charbuf = unsafe {
        if let Some(charbuf) = STDOUT.as_mut() {
            charbuf
        } else {
            STDOUT.insert(OutputBuffer::new(1))
        }
    };

    charbuf.puts(ch)
}

pub(crate) fn eputs<T: AsRef<[u8]>>(ch: T) -> usize {
    let charbuf = unsafe {
        if let Some(charbuf) = STDERR.as_mut() {
            charbuf
        } else {
            STDERR.insert(OutputBuffer::new(2))
        }
    };

    charbuf.puts(ch)
}

pub(crate) fn getc() -> Option<char> {
    flush();
    let charbuf = unsafe {
        if let Some(charbuf) = STDIN.as_mut() {
            charbuf
        } else {
            STDIN.insert(InputBuffer::new(0))
        }
    };

    charbuf.getc().map(|xs| xs as char)
}

pub(crate) fn flush() {
    unsafe {
        STDERR.as_mut().map(|xs| xs.flush_all());
        STDOUT.as_mut().map(|xs| xs.flush_all());
    }
}

pub(crate) fn itoa(input: u32) -> &'static str {
    static mut OUTBUF: [u8; 16] = [0; 16];

    if input == 0 {
        unsafe { OUTBUF[0] = b'0' };
        return unsafe { ::core::str::from_utf8_unchecked(&OUTBUF[0..1]) }
    }

    let mut input = input;
    let mut idx = unsafe { OUTBUF.len() } - 1;
    while input > 0 {
        let ch = (input % 10) as u8 + b'0';
        unsafe { OUTBUF[idx] = ch };
        input /= 10;
        idx -= 1;
    }

    unsafe { ::core::str::from_utf8_unchecked(&OUTBUF[idx + 1..]) }
}
