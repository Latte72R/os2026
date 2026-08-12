const USER_ENTRY: usize = 0x8070_0000;
const USER_REGION_END: usize = 0x8080_0000;

static USER_IMAGE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/riscv64imac-unknown-none-elf/release/shell.bin"
));

pub fn load() {
    assert!(USER_IMAGE.len() <= USER_REGION_END - USER_ENTRY);

    // SAFETY: The user linker reserves this physical range, it does not
    // overlap the kernel heap, and the source is a live static byte slice.
    unsafe {
        core::ptr::copy_nonoverlapping(
            USER_IMAGE.as_ptr(),
            USER_ENTRY as *mut u8,
            USER_IMAGE.len(),
        );
    }
}

pub const fn entry() -> usize {
    USER_ENTRY
}
