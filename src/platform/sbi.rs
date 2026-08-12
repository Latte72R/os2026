#[inline]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> (isize, isize) {
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

pub fn getchar() -> Option<u8> {
    const SBI_CONSOLE_GETCHAR_EID: usize = 0x02;
    let (value, _) = sbi_call(SBI_CONSOLE_GETCHAR_EID, 0, 0, 0, 0);

    (value >= 0).then_some(value as u8)
}

#[allow(dead_code)]
pub fn shutdown(success: bool) -> ! {
    const SBI_SYSTEM_RESET_EID: usize = 0x53525354;
    const SBI_SYSTEM_RESET_FID: usize = 0;
    const RESET_TYPE_SHUTDOWN: usize = 0;
    const RESET_REASON_NONE: usize = 0;
    const RESET_REASON_SYSTEM_FAILURE: usize = 1;

    let reason = if success {
        RESET_REASON_NONE
    } else {
        RESET_REASON_SYSTEM_FAILURE
    };

    let _ = sbi_call(
        SBI_SYSTEM_RESET_EID,
        SBI_SYSTEM_RESET_FID,
        RESET_TYPE_SHUTDOWN,
        reason,
        0,
    );

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
