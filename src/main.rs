#![no_std]
#![no_main]

use vertos;
use vertos::init;

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    init::init_basic_runtime();
    vertos::info!("vertos kernel started in S-mode (satp.MODE=Bare)");

    vertos::user_image::load();
    let shell_pid =
        vertos::process::create_user_process(0).expect("failed to create user shell process");
    vertos::info!("starting U-mode shell as PID {shell_pid}");

    vertos::process::yield_process();

    loop {
        vertos::process::yield_process();
    }
}
