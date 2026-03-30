use crate::csr::{read_scause, read_sepc, read_stval, write_sepc, write_stvec};
use crate::error;
use core::arch::global_asm;

global_asm!(include_str!("../boot/trap.S"));

unsafe extern "C" {
    fn kernel_entry_trap();
}

pub fn init_trap() {
    let addr = kernel_entry_trap as *const () as usize;
    write_stvec(addr);
}

#[unsafe(no_mangle)]
extern "C" fn handle_trap() {
    let scause = read_scause();
    let sepc = read_sepc();
    let stval = read_stval();

    error!("trap occurred!");
    error!("scause = {:#x}", scause);
    error!("sepc   = {:#x}", sepc);
    error!("stval  = {:#x}", stval);

    // Skip the faulting 32-bit instruction to avoid trapping on the same PC forever.
    write_sepc(sepc + 4);
}
