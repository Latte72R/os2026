use crate::arch::csr::{read_scause, read_sepc, read_stval, write_stvec};
use crate::error;
use core::arch::global_asm;

global_asm!(include_str!("../../boot/trap.S"));

#[repr(C)]
pub struct TrapFrame {
    pub ra: usize,
    pub gp: usize,
    pub tp: usize,

    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,

    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,

    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,

    pub sp: usize,
}

// TrapFrame のサイズが想定どおりかをコンパイル時に検査する
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 31 * core::mem::size_of::<usize>());

unsafe extern "C" {
    fn kernel_entry_trap();
}

pub fn init_trap() {
    let addr = kernel_entry_trap as *const () as usize;
    write_stvec(addr);
}

#[unsafe(no_mangle)]
extern "C" fn handle_trap(_frame: &mut TrapFrame) {
    let scause = read_scause();
    let sepc = read_sepc();
    let stval = read_stval();

    error!("trap occurred!");
    error!("scause = {:#x}", scause);
    error!("sepc   = {:#x}", sepc);
    error!("stval  = {:#x}", stval);

    // just panic on any trap
    panic!(
        "unexpected trap: scause={:#x}, sepc={:#x}, stval={:#x}",
        scause, sepc, stval
    );
}
