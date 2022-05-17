#![no_std]
#![feature(lang_items)]
#![no_main]

mod io;
mod sys;

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
pub unsafe extern "C" fn main() {
    puts("hello world\n");
    r#match('\0');
    expr();
    putc('\n');
    exit(0);
}
