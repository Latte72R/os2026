use super::ALLOCATOR;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::slice;

pub const PAGE_SIZE: usize = 4096;

pub struct Pages {
    ptr: NonNull<u8>,
    count: usize,
}

impl Pages {
    pub fn alloc(count: usize) -> Option<Self> {
        if count == 0 {
            return None;
        }

        let size = PAGE_SIZE.checked_mul(count)?;
        let layout = Layout::from_size_align(size, PAGE_SIZE).ok()?;

        let ptr = ALLOCATOR.alloc_with_options(layout);
        let ptr = NonNull::new(ptr)?;

        let mut pages = Self { ptr, count };
        pages.fill_with_bytes(0);

        Some(pages)
    }

    pub fn fill_with_bytes(&mut self, value: u8) {
        // SAFETY:
        // `ptr` points to a writable allocation of `size` bytes
        // returned by the global allocator.
        unsafe {
            core::ptr::write_bytes(self.ptr.as_ptr(), value, self.size());
        }
    }

    pub fn start_address(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn page_count(&self) -> usize {
        self.count
    }

    pub fn size(&self) -> usize {
        PAGE_SIZE * self.count
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY:
        // `ptr` is valid for `self.size()` bytes for the lifetime
        // of this Pages value. Shared access does not permit mutation.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.size()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY:
        // `&mut self` guarantees exclusive access to the allocation.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size()) }
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size(), PAGE_SIZE)
            .expect("Pages always has a valid layout");

        // SAFETY:
        // `ptr` was allocated by `ALLOCATOR` with the same layout,
        // and this Pages value uniquely owns the allocation.
        unsafe {
            GlobalAlloc::dealloc(&ALLOCATOR, self.ptr.as_ptr(), layout);
        }
    }
}

impl core::fmt::Debug for Pages {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Pages {{ range: {:#018x}..{:#018x}, count: {} }}",
            self.start_address(),
            self.start_address() + self.size(),
            self.count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn alloc_pages_are_aligned_and_zeroed() {
        let pages = Pages::alloc(2).expect("failed to allocate pages");

        assert_eq!(pages.start_address() % PAGE_SIZE, 0);
        assert_eq!(pages.page_count(), 2);
        assert_eq!(pages.size(), PAGE_SIZE * 2);
        assert!(pages.as_slice().iter().all(|byte| *byte == 0));
    }

    #[test_case]
    fn allocated_pages_do_not_overlap() {
        let pages1 = Pages::alloc(2).expect("failed to allocate pages");
        let pages2 = Pages::alloc(1).expect("failed to allocate pages");

        let pages1_start = pages1.start_address();
        let pages1_end = pages1_start + pages1.size();

        let pages2_start = pages2.start_address();
        let pages2_end = pages2_start + pages2.size();

        assert!(pages1_end <= pages2_start || pages2_end <= pages1_start);
    }

    #[test_case]
    fn pages_can_be_written() {
        let mut pages = Pages::alloc(1).expect("failed to allocate pages");

        pages.as_mut_slice()[0] = 0x12;
        pages.as_mut_slice()[PAGE_SIZE - 1] = 0x34;

        assert_eq!(pages.as_slice()[0], 0x12);
        assert_eq!(pages.as_slice()[PAGE_SIZE - 1], 0x34);
    }
}
