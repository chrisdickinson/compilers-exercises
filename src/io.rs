use crate::sys::syscall3;

static mut STDIN: Option<InputBuffer<4096>> = None;

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
}

impl<const N: usize> InputBuffer<N> {
    const MIDPOINT: usize = if N & (N - 1) != 0 {
        panic!("N must be a power of 2")
    } else if N == usize::max_value() {
        panic!("N must be < usize::max_value")
    } else {
        N >> 1
    };

    fn new() -> Self {
        let mut s = Self {
            buf: [0u8; N],
            cursor: 0,
            eofidx: N + 1,
        };

        s.fill();
        s
    }

    fn fill(&mut self) {
        let mem = &mut self.buf[self.cursor..self.cursor + Self::MIDPOINT];
        let read_bytes = unsafe { syscall3(3, 0, mem.as_mut_ptr() as usize, Self::MIDPOINT) };
        if read_bytes < Self::MIDPOINT {
            self.eofidx = self.cursor + read_bytes + 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        let answer = self.getc();
        self.ungetc();
        answer
    }

    fn getc(&mut self) -> Option<char> {
        let ch = self.buf[self.cursor] as char;
        let last = self.cursor;
        self.cursor = (self.cursor + 1) & N - 1;

        if self.cursor == 0 || self.cursor == Self::MIDPOINT {
            self.fill();
        }

        if last != self.eofidx {
            Some(ch)
        } else {
            None
        }
    }

    fn ungetc(&mut self) {
        self.cursor = if self.cursor == 0 {
            N - 1
        } else {
            self.cursor - 1
        };
    }
}

pub(crate) fn putc(ch: char) -> usize {
    let chara = [ch];
    unsafe { syscall3(4, 1, chara.as_ptr() as usize, 1) }
}

#[allow(dead_code)]
pub(crate) fn puts<T: AsRef<[u8]>>(ch: T) -> usize {
    let ch = ch.as_ref();
    unsafe { syscall3(4, 1, ch.as_ptr() as usize, ch.len()) }
}

pub(crate) fn getc() -> Option<char> {
    let charbuf = unsafe {
        if let Some(charbuf) = STDIN.as_mut() {
            charbuf
        } else {
            STDIN.insert(InputBuffer::new())
        }
    };

    charbuf.getc()
}

pub(crate) fn itoa(input: u32) -> &'static str {
    static mut OUTBUF: [u8; 256] = [0; 256];

    let mut input = input;
    let mut idx = 255;
    while input > 0 {
        let ch = (input % 10) as u8 + b'0';
        unsafe { OUTBUF[idx] = ch };
        input /= 10;
        idx -= 1;
    }

    unsafe { ::core::str::from_utf8_unchecked(&OUTBUF[idx + 1..]) }
}
