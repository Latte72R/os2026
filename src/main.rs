#![no_std]
#![no_main]

use vertos;
use vertos::init;

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    init::init_basic_runtime();
    vertos::info!("minimal kernel started.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
