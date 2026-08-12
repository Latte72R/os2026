extern crate alloc;

use alloc::alloc::GlobalAlloc;
use alloc::alloc::Layout;
use alloc::boxed::Box;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::cmp::max;
use core::fmt;
use core::mem::size_of;
use core::ops::DerefMut;
use core::ptr::null_mut;

unsafe extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    debug_assert!(align.is_power_of_two());

    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
}

pub fn round_up_to_header_align(v: usize) -> Option<usize> {
    let header_align = core::mem::align_of::<Header>();
    align_up(v, header_align)
}

struct Header {
    next_header: Option<Box<Header>>,
    size: usize,
    is_allocated: bool,
    _reserved: usize,
}

const HEADER_SIZE: usize = size_of::<Header>();

#[allow(clippy::assertions_on_constants)]
const _: () = assert!(HEADER_SIZE == 32);

const _: () = assert!(HEADER_SIZE.count_ones() == 1);

impl Header {
    fn can_provide(&self, size: usize, align: usize) -> bool {
        self.size >= size + HEADER_SIZE * 2 + align
    }
    fn is_allocated(&self) -> bool {
        self.is_allocated
    }
    fn end_addr(&self) -> usize {
        self as *const Header as usize + self.size
    }
    unsafe fn new_from_addr(addr: usize) -> Box<Header> {
        let header = addr as *mut Header;

        // SAFETY:
        // The caller guarantees that `addr` is properly aligned,
        // writable for `HEADER_SIZE` bytes, and not currently occupied
        // by another live value.
        unsafe {
            header.write(Header {
                next_header: None,
                size: 0,
                is_allocated: false,
                _reserved: 0,
            });
            Box::from_raw(addr as *mut Header)
        }
    }
    unsafe fn from_allocated_region(addr: *mut u8) -> Box<Header> {
        // SAFETY:
        // The caller guarantees that `addr` was returned by this
        // allocator and is preceded by a valid Header.
        unsafe {
            let header = addr.sub(HEADER_SIZE) as *mut Header;
            Box::from_raw(header)
        }
    }
    fn provide(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let size = max(round_up_to_header_align(size)?, HEADER_SIZE);
        let align = max(align, HEADER_SIZE);
        if self.is_allocated() {
            return None;
        }

        let current_payload = self as *mut Header as usize + HEADER_SIZE;
        let current_payload_is_aligned = current_payload.is_multiple_of(align);
        let current_region_is_large_enough = self.size >= HEADER_SIZE + size;

        if !self.can_provide(size, align) {
            if current_payload_is_aligned && current_region_is_large_enough {
                self.is_allocated = true;
                return Some(current_payload as *mut u8);
            }
            return None;
        }

        {
            let mut size_used = 0;
            let allocated_addr = (self.end_addr() - size) & !(align - 1);
            let mut header_for_allocated =
                unsafe { Self::new_from_addr(allocated_addr - HEADER_SIZE) };
            header_for_allocated.is_allocated = true;
            header_for_allocated.size = size + HEADER_SIZE;
            size_used += header_for_allocated.size;
            header_for_allocated.next_header = self.next_header.take();
            if header_for_allocated.end_addr() != self.end_addr() {
                let mut header_for_padding =
                    unsafe { Self::new_from_addr(header_for_allocated.end_addr()) };
                header_for_padding.is_allocated = false;
                header_for_padding.size = self.end_addr() - header_for_allocated.end_addr();
                size_used += header_for_padding.size;
                header_for_padding.next_header = header_for_allocated.next_header.take();
                header_for_allocated.next_header = Some(header_for_padding);
            }
            assert!(self.size >= size_used + HEADER_SIZE);
            self.size -= size_used;
            self.next_header = Some(header_for_allocated);
            Some(allocated_addr as *mut u8)
        }
    }
}

impl Drop for Header {
    fn drop(&mut self) {
        panic!("Header should not be dropped!");
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header @ {:#018X} {{ size: {:#018X}, is_allocated: {} }}",
            self as *const Header as usize,
            self.size,
            self.is_allocated()
        )
    }
}

pub struct FirstFitAllocator {
    first_header: RefCell<Option<Box<Header>>>,
}

#[global_allocator]
pub static ALLOCATOR: FirstFitAllocator = FirstFitAllocator {
    first_header: RefCell::new(None),
};

// 非同期処理には非対応
unsafe impl Sync for FirstFitAllocator {}

unsafe impl GlobalAlloc for FirstFitAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_with_options(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY:
        // GlobalAlloc requires `ptr` to have been allocated by this
        // allocator and not to have been deallocated already.
        let mut region = unsafe { Header::from_allocated_region(ptr) };
        region.is_allocated = false;
        Box::leak(region);
    }
}

impl FirstFitAllocator {
    pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
        let mut header = self.first_header.borrow_mut();
        let mut header = header.deref_mut();
        loop {
            match header {
                Some(e) => match e.provide(layout.size(), layout.align()) {
                    Some(p) => break p,
                    None => {
                        header = e.next_header.borrow_mut();
                        continue;
                    }
                },
                None => {
                    break null_mut::<u8>();
                }
            }
        }
    }
    pub unsafe fn init(&self) {
        let start = core::ptr::addr_of!(__heap_start) as usize;
        let end = core::ptr::addr_of!(__heap_end) as usize;

        unsafe {
            self.add_free_region(start, end - start);
        }
    }
    pub unsafe fn add_free_region(&self, mut start: usize, mut size: usize) {
        // 空き領域をリンクリストへ追加
        if start == 0 {
            start += 4096;
            size = size.saturating_sub(4096);
        }
        if size <= 4096 {
            return;
        }
        let mut header = unsafe { Header::new_from_addr(start) };
        header.next_header = None;
        header.is_allocated = false;
        header.size = size;
        let mut first_header = self.first_header.borrow_mut();
        let prev_last = first_header.replace(header);
        drop(first_header);
        let mut header = self.first_header.borrow_mut();
        header.as_mut().unwrap().next_header = prev_last;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test_case]
    fn malloc_iterate_free_and_alloc() {
        use alloc::vec::Vec;
        for i in 0..256 {
            let mut vec = Vec::new();
            vec.resize(i, 10);
        }
    }

    #[test_case]
    fn malloc_align() {
        let mut pointers = [null_mut::<u8>(); 100];
        for align in [1, 2, 4, 8, 16, 4096] {
            let layout = Layout::from_size_align(1234, align).expect("Failed to create Layout");
            for e in pointers.iter_mut() {
                *e = ALLOCATOR.alloc_with_options(layout);
                assert!(*e as usize != 0);
                assert!((*e as usize) % align == 0);
            }
            for pointer in pointers {
                unsafe {
                    ALLOCATOR.dealloc(pointer, layout);
                }
            }
        }
    }

    #[test_case]
    fn malloc_align_random_order() {
        for align in [32, 4096, 8, 4, 16, 2, 1] {
            let mut pointers = [null_mut::<u8>(); 100];
            let layout = Layout::from_size_align(1234, align).expect("Failed to create Layout");
            for e in pointers.iter_mut() {
                *e = ALLOCATOR.alloc_with_options(layout);
                assert!(*e as usize != 0);
                assert!((*e as usize) % align == 0);
            }
            for pointer in pointers {
                unsafe {
                    ALLOCATOR.dealloc(pointer, layout);
                }
            }
        }
    }

    #[test_case]
    fn allocated_objects_have_no_overlap() {
        let allocations = [
            Layout::from_size_align(128, 128).unwrap(),
            Layout::from_size_align(32, 32).unwrap(),
            Layout::from_size_align(8, 8).unwrap(),
            Layout::from_size_align(16, 16).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(4, 4).unwrap(),
            Layout::from_size_align(2, 2).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(64, 64).unwrap(),
            Layout::from_size_align(1, 1).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(600000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
        ];
        let mut pointers = vec![null_mut::<u8>(); allocations.len()];
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            for k in 0..layout.size() {
                unsafe { *pointer.add(k) = i as u8 }
            }
        }
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (_, (layout, pointer)) = e;
            unsafe { ALLOCATOR.dealloc(*pointer, *layout) }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            for k in 0..layout.size() {
                unsafe { *pointer.add(k) = i as u8 }
            }
        }
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
        for (layout, pointer) in allocations.iter().zip(pointers) {
            unsafe {
                ALLOCATOR.dealloc(pointer, *layout);
            }
        }
    }
}
