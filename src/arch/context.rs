use crate::memory::Pages;
use core::arch::global_asm;
use core::mem::size_of;

global_asm!(include_str!("../../boot/context.S"));
global_asm!(include_str!("../../boot/user.S"));

unsafe extern "C" {
    fn switch_context_asm(previous_sp: *mut usize, next_sp: *const usize);
    fn enter_user_asm(
        entry: usize,
        user_stack_top: usize,
        argument: usize,
        kernel_stack_top: usize,
    ) -> !;
}

/// Enters a user program for the first time through `sret`.
///
/// # Safety
///
/// All addresses must point to live memory in the shared Bare address space,
/// and `kernel_stack_top` must remain valid for the lifetime of the process.
pub unsafe fn enter_user(
    entry: usize,
    user_stack_top: usize,
    argument: usize,
    kernel_stack_top: usize,
) -> ! {
    unsafe {
        enter_user_asm(entry, user_stack_top, argument, kernel_stack_top);
    }
}

/// Switches from the current execution context to another context.
///
/// # Safety
///
/// - `previous_sp` must point to writable and stable storage.
/// - `next_sp` must point to a valid saved stack pointer.
/// - The stack referenced by `next_sp` must contain the register layout
///   expected by `context.S`.
/// - Both pointers and their owning processes must remain valid until this
///   context is switched back.
pub unsafe fn switch_context(previous_sp: *mut usize, next_sp: *const usize) {
    unsafe {
        switch_context_asm(previous_sp, next_sp);
    }
}

const CONTEXT_SLOTS: usize = 16;
pub const CONTEXT_SIZE: usize = CONTEXT_SLOTS * core::mem::size_of::<usize>();

#[repr(C)]
struct ContextFrame {
    ra: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
    padding: [usize; 3],
}
const _: () = assert!(size_of::<ContextFrame>() == CONTEXT_SIZE);

impl ContextFrame {
    fn new(entry: extern "C" fn() -> !) -> Self {
        Self {
            ra: entry as usize,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            padding: [0; 3],
        }
    }
}

pub fn initialize_stack(stack: &mut Pages, entry: extern "C" fn() -> !) -> Option<usize> {
    let stack_start = stack.start_address();
    let stack_top = stack_start.checked_add(stack.size())?;
    let initial_sp = stack_top.checked_sub(CONTEXT_SIZE)?;

    if stack_top % 16 != 0 {
        return None;
    }

    if initial_sp % 16 != 0 {
        return None;
    }

    let frame = ContextFrame::new(entry);

    // SAFETY:
    // - initial_sp is inside the allocation owned by stack.
    // - CONTEXT_SIZE bytes are available between initial_sp and stack_top.
    // - initial_sp is aligned for ContextFrame.
    // - &mut Pages guarantees exclusive access to the allocation.
    unsafe {
        (initial_sp as *mut ContextFrame).write(frame);
    }

    Some(initial_sp)
}
