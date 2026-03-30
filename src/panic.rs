use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::error!("panic: {}", info);

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
