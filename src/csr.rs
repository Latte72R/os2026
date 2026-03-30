#[inline]
pub fn read_scause() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!(
            "csrr {0}, scause",
            out(reg) value,
        );
    }
    value
}

#[inline]
pub fn read_sepc() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!(
            "csrr {0}, sepc",
            out(reg) value,
        );
    }
    value
}

#[inline]
pub fn read_stval() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!(
            "csrr {0}, stval",
            out(reg) value,
        );
    }
    value
}

#[inline]
pub fn write_stvec(value: usize) {
    unsafe {
        core::arch::asm!("csrw stvec, {0}", in(reg) value);
    }
}

#[inline]
pub fn write_sepc(value: usize) {
    unsafe {
        core::arch::asm!("csrw sepc, {0}", in(reg) value);
    }
}
