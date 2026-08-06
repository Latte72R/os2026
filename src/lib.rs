#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod arch;
pub mod init;
pub mod memory;
pub mod panic;
pub mod platform;
pub mod print;

#[cfg(test)]
mod test_runner;

#[unsafe(no_mangle)]
#[cfg(test)]
extern "C" fn rust_main() {
    init::init_basic_runtime();
    test_main();
}
