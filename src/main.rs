#![no_std]
#![no_main]

mod panic;
mod print;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("../boot/entry.S"));

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    clear_bss();

    info!("minimal kernel started.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

fn clear_bss() {
    unsafe {
        let start = core::ptr::addr_of_mut!(__bss_start);
        let end = core::ptr::addr_of_mut!(__bss_end);
        let size = end.offset_from(start) as usize;
        core::ptr::write_bytes(start, 0, size);
    }
}
