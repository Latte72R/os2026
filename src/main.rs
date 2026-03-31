#![no_std]
#![no_main]

use os2026;
use os2026::init;

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    init::init_basic_runtime();
    os2026::info!("minimal kernel started.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
