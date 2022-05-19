#![no_std]
#![feature(lang_items)]
#![no_main]

mod io;
mod sys;
mod regex;

use crate::io::{getc, putc, puts};
use crate::sys::exit;

/* expr ::= expr + term | expr - term | term
 * term ::= 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9
 *
 *
 * expr -> term rest
 * rest -> + expr | - expr | ε
 * term -> 0..9
 */

static mut LOOKAHEAD: char = '\0';

fn expr() {
    term();
    rest();
}

fn rest() {
    loop {
        match lookahead() {
            '+' => {
                r#match('+');
                term();
                putc('+');
            }
            '-' => {
                r#match('-');
                term();
                putc('-');
            }
            _ => break,
        }
    }
}

fn lookahead() -> char {
    unsafe { LOOKAHEAD }
}

fn r#match(ch: char) {
    if lookahead() == ch {
        if let Some(chara) = getc() {
            unsafe {
                LOOKAHEAD = chara;
            }
        } else {
            error()
        }
    } else {
        error()
    }
}

fn error() {
    panic!("\x1b[31merror\x1b[0m");
}

fn term() {
    match lookahead() {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
            putc(lookahead());
            r#match(lookahead())
        }
        _ => error(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const char) {

    use crate::regex::NFA;
    use crate::io::itoa;

    #[cfg(any())]
    {
        // empty
        let nfa = NFA::<256>::from_regex_bytes(b"");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // single char
        let nfa = NFA::<256>::from_regex_bytes(b"a");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // product
        let nfa = NFA::<256>::from_regex_bytes(b"ab");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // product * 5
        let nfa = NFA::<256>::from_regex_bytes(b"apple");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // kleene star
        let nfa = NFA::<256>::from_regex_bytes(b"a*");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // product + kleene star on last char
        let nfa = NFA::<256>::from_regex_bytes(b"apple*");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // multiple stars, products
        let nfa = NFA::<256>::from_regex_bytes(b"ap*le*");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // alternate
        let nfa = NFA::<256>::from_regex_bytes(b"apple|banana");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // alternate, star on last a
        let nfa = NFA::<256>::from_regex_bytes(b"apple|banana*");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // alternate, interstital stars
        let nfa = NFA::<256>::from_regex_bytes(b"ap*le|bana*na");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // multiple alternate
        let nfa = NFA::<256>::from_regex_bytes(b"apple|banana|cat");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        // group concat
        let nfa = NFA::<256>::from_regex_bytes(b"wow(apple)cat");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(all())]
    {
        // group alternate concat
        let nfa = NFA::<256>::from_regex_bytes(b"(apple|banana) cat");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        let nfa = NFA::<256>::from_regex_bytes(b"(apple|banana)*");
        nfa.debug_print();
        exit(1);
    }

    #[cfg(any())]
    {
        let nfa = NFA::<256>::from_regex_bytes(b"(apple|banana)|cat");
        nfa.debug_print();
        exit(1);
    }

    exit(1);
    puts("hello world\n");
    r#match('\0');
    expr();
    putc('\n');
    exit(0);
}
