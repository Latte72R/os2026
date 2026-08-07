#![no_std]
#![no_main]

use vertos;
use vertos::init;

extern "C" fn process_a_entry() -> isize {
    for i in 0..3 {
        vertos::info!("A: {i}");
        vertos::process::yield_process();
    }
    0
}

extern "C" fn process_b_entry() -> isize {
    for i in 0..3 {
        vertos::info!("B: {i}");
        vertos::process::yield_process();
    }
    0
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

    vertos::info!(
        "PID {pid_a} state: {:?}",
        vertos::process::process_state(pid_a)
    );
    vertos::info!(
        "PID {pid_b} state: {:?}",
        vertos::process::process_state(pid_b)
    );

    vertos::info!("processes finished, halting CPU.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
