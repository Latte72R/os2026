use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    panic!("allocation failed");
}

unsafe extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
}

pub struct BumpAllocator {
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            next: UnsafeCell::new(0),
        }
    }

    fn heap_range() -> (usize, usize) {
        let start = core::ptr::addr_of!(__heap_start) as usize;
        let end = core::ptr::addr_of!(__heap_end) as usize;
        (start, end)
    }

    pub unsafe fn init(&self) {
        let (start, _) = Self::heap_range();
        unsafe { *self.next.get() = start };
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (heap_start, heap_end) = Self::heap_range();
        let next = self.next.get();

        let current = unsafe { if *next == 0 { heap_start } else { *next } };
        let aligned = (current + layout.align() - 1) & !(layout.align() - 1);

        let new_next = match aligned.checked_add(layout.size()) {
            Some(v) => v,
            None => return null_mut(),
        };

        if new_next > heap_end {
            return null_mut();
        }

        unsafe { *next = new_next };
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[test_case]
fn test_bump_allocator() {
    extern crate alloc;
    use alloc::boxed::Box;

    let x = Box::new(42usize);
    crate::info!("boxed value = {}", *x);
}
