#[cfg(not(test))]
use crate::io::{itoa, eputs};
use crate::io::flush;

#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn syscall3(syscall_number: u64, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut arg0 = arg0;
    core::arch::asm!(
        "svc 0",
        in("x16") syscall_number,
        inout("x0") arg0,
        in("x1") arg1,
        in("x2") arg2,
        options(nostack)
    );
    arg0
}

#[cfg(target_arch = "aarch64")]
pub(crate) fn exit(code: i32) -> ! {
    flush();
    let syscall_number: u64 = 1;
    unsafe {
        core::arch::asm!(
            "svc 0",
            in("x16") syscall_number,
            in("x0") code,
            options(noreturn)
        );
    }
}

#[cfg(not(test))]
#[lang = "eh_personality"]
fn eh_personality() {}

#[cfg(not(test))]
#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    flush();
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        eputs("panic: \"");
        eputs(s);
        eputs("\"\n");
    }

    if let Some(loc) = panic_info.location() {
        eputs("panic: in \"");
        eputs(loc.file());
        eputs("\" at line ");
        eputs(itoa(loc.line()));
        eputs("\n");
    }

    exit(1);
}
