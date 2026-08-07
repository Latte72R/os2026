#![no_std]
#![no_main]

use vertos;
use vertos::init;

extern "C" fn process_a_entry() -> ! {
    loop {
        vertos::info!("A");
        vertos::process::yield_process();
    }
}

extern "C" fn process_b_entry() -> ! {
    loop {
        vertos::info!("B");
        vertos::process::yield_process();
    }
}

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    init::init_basic_runtime();
    vertos::info!("minimal kernel started.");

    let pid_a =
        vertos::process::create_process(process_a_entry).expect("failed to create process A");

    let pid_b =
        vertos::process::create_process(process_b_entry).expect("failed to create process B");
    vertos::info!("created PID {pid_a}");
    vertos::info!("created PID {pid_b}");

    vertos::process::yield_process();

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
