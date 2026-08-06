use crate::arch::trap::init_trap;
use crate::memory::BumpAllocator;

use core::arch::global_asm;

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

global_asm!(include_str!("../boot/entry.S"));

unsafe extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

pub fn init_basic_runtime() {
    clear_bss();

    init_trap();

    unsafe {
        ALLOCATOR.init();
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
