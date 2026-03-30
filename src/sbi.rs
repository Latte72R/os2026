#[inline]
fn sbi_call(
    eid: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> (isize, isize) {
    let error: isize;
    let value: isize;

    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 as isize => error,
            inlateout("a1") arg1 as isize => value,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
        );
    }

    (error, value)
}

pub fn putchar(ch: u8) {
    const SBI_CONSOLE_PUTCHAR_EID: usize = 0x01;
    let _ = sbi_call(SBI_CONSOLE_PUTCHAR_EID, 0, ch as usize, 0, 0);
}
