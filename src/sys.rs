use crate::io::{itoa, puts};

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

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    if let Some(ref s) = panic_info.payload().downcast_ref::<&str>() {
        puts("panic: \"");
        puts(s);
        puts("\"\n");
    }

    if let Some(loc) = panic_info.location() {
        puts("panic: in \"");
        puts(loc.file());
        puts("\" at line ");
        puts(itoa(loc.line()));
        puts("\n");
    }

    exit(1);
}
