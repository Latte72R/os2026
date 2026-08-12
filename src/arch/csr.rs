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
pub fn read_stvec() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!(
            "csrr {value}, stvec",
            value = out(reg) value,
        );
    }

    value
}

#[inline]
pub fn write_sepc(value: usize) {
    unsafe {
        core::arch::asm!("csrw sepc, {0}", in(reg) value);
    }
}

#[inline]
pub fn read_sscratch() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!(
            "csrr {value}, sscratch",
            value = out(reg) value,
        );
    }

    value
}

#[inline]
pub fn write_sscratch(value: usize) {
    unsafe {
        core::arch::asm!(
            "csrw sscratch, {value}",
            value = in(reg) value,
        );
    }
}

#[inline]
pub fn read_satp() -> usize {
    let value: usize;

    unsafe {
        core::arch::asm!(
            "csrr {value}, satp",
            value = out(reg) value,
        );
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn sscratch_can_be_read_and_written() {
        let original = read_sscratch();

        for expected in [0, 1, 0x1234_5678, usize::MAX] {
            write_sscratch(expected);
            let actual = read_sscratch();
            write_sscratch(original);
            assert_eq!(actual, expected);
        }

        assert_eq!(read_sscratch(), original);
    }

    #[test_case]
    fn sepc_can_be_read_and_written() {
        let original = read_sepc();

        for expected in [0, 0x1000, 0x1234_5678] {
            write_sepc(expected);
            let actual = read_sepc();

            write_sepc(original);

            assert_eq!(actual, expected);
        }

        assert_eq!(read_sepc(), original);
    }

    #[test_case]
    fn stvec_is_initialized_in_direct_mode() {
        let stvec = read_stvec();

        const STVEC_MODE_MASK: usize = 0b11;
        const STVEC_MODE_DIRECT: usize = 0;

        let mode = stvec & STVEC_MODE_MASK;
        let base = stvec & !STVEC_MODE_MASK;

        // trap::init()によってDirect modeで初期化されている。
        assert_eq!(mode, STVEC_MODE_DIRECT);

        // trap entryのアドレスが設定されている。
        assert_ne!(base, 0);

        // stvecのBASEは4-byte alignedである。
        assert_eq!(base % 4, 0);
    }

    #[test_case]
    fn paging_remains_disabled() {
        assert_eq!(read_satp(), 0);
    }
}
