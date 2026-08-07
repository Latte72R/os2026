#![no_std]
#![no_main]

use vertos;
use vertos::executor::Executor;
use vertos::executor::ROOT_EXECUTOR;
use vertos::executor::Task;
use vertos::executor::yield_execution;
use vertos::init;

#[unsafe(no_mangle)]
extern "C" fn rust_main() -> ! {
    init::init_basic_runtime();
    vertos::info!("minimal kernel started.");
    let task1 = Task::new(async {
        for i in 0..=3 {
            vertos::info!("1-{i}");
            yield_execution().await;
        }
        Ok(())
    });
    let task2 = Task::new(async {
        for i in 0..=3 {
            vertos::info!("2-{i}");
            yield_execution().await;
        }
        Ok(())
    });
    {
        let mut executor = ROOT_EXECUTOR.lock();
        executor.spawn(task1);
        executor.spawn(task2);
    }
    Executor::run(&ROOT_EXECUTOR);

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
