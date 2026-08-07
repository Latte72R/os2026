extern crate alloc;

use super::{PAGE_SIZE, Pages};
use alloc::vec::Vec;

const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;
pub const PTE_U: u64 = 1 << 4;
const PTE_A: u64 = 1 << 6;
const PTE_D: u64 = 1 << 7;

#[derive(Debug)]
pub struct AddressSpace {
    root: Pages,
    tables: Vec<Pages>,
}

impl AddressSpace {
    pub fn new() -> Option<Self> {
        Some(Self {
            root: Pages::alloc(1)?,
            tables: Vec::new(),
        })
    }

    pub fn root_address(&self) -> usize {
        self.root.start_address()
    }

    fn alloc_table(&mut self) -> Option<usize> {
        let table = Pages::alloc(1)?;
        let address = table.start_address();

        self.tables.push(table);

        Some(address)
    }

    pub fn map_page(
        &mut self,
        virtual_address: usize,
        physical_address: usize,
        flags: u64,
    ) -> Option<()> {
        assert_eq!(virtual_address % PAGE_SIZE, 0);
        assert_eq!(physical_address % PAGE_SIZE, 0);

        let mut table = self.root_address();

        for level in [2, 1] {
            let index = vpn(virtual_address, level);
            let mut entry = read_pte(table, index);

            if entry & PTE_V == 0 {
                let child = self.alloc_table()?;
                entry = make_pte(child, PTE_V);
                write_pte(table, index, entry);
            }

            table = pte_address(entry);
        }

        let index = vpn(virtual_address, 0);

        assert_eq!(
            read_pte(table, index) & PTE_V,
            0,
            "virtual page is already mapped",
        );

        write_pte(
            table,
            index,
            make_pte(physical_address, flags | PTE_V | PTE_A | PTE_D),
        );

        Some(())
    }
}

fn vpn(virtual_address: usize, level: usize) -> usize {
    (virtual_address >> (12 + level * 9)) & 0x1ff
}

fn make_pte(physical_address: usize, flags: u64) -> u64 {
    ((physical_address as u64 / PAGE_SIZE as u64) << 10) | flags
}

fn pte_address(entry: u64) -> usize {
    ((entry >> 10) * PAGE_SIZE as u64) as usize
}

fn read_pte(table: usize, index: usize) -> u64 {
    // SAFETY: `table` points to a 4 KiB page table and the Sv39
    // index is in the range 0..512.
    unsafe { (table as *const u64).add(index).read() }
}

fn write_pte(table: usize, index: usize, value: u64) {
    // SAFETY: `table` points to a writable 4 KiB page table and the
    // Sv39 index is in the range 0..512.
    unsafe {
        (table as *mut u64).add(index).write(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn address_space_has_aligned_root_table() {
        let address_space = AddressSpace::new().expect("failed to create address space");

        assert_eq!(address_space.root_address() % PAGE_SIZE, 0);
        assert_eq!(address_space.tables.len(), 0);
    }

    #[test_case]
    fn page_can_be_mapped() {
        const USER_ADDRESS: usize = 0x0100_0000;

        let page = Pages::alloc(1).expect("failed to allocate page");

        let physical_address = page.start_address();

        let mut address_space = AddressSpace::new().expect("failed to create address space");

        address_space
            .map_page(USER_ADDRESS, physical_address, PTE_R | PTE_W | PTE_U)
            .expect("failed to map page");

        assert_eq!(address_space.tables.len(), 2);

        let level2 = read_pte(address_space.root_address(), vpn(USER_ADDRESS, 2));

        let level1 = read_pte(pte_address(level2), vpn(USER_ADDRESS, 1));

        let level0 = read_pte(pte_address(level1), vpn(USER_ADDRESS, 0));

        assert_ne!(level2 & PTE_V, 0);
        assert_ne!(level1 & PTE_V, 0);
        assert_ne!(level0 & PTE_V, 0);

        assert_ne!(level0 & PTE_R, 0);
        assert_ne!(level0 & PTE_W, 0);
        assert_ne!(level0 & PTE_U, 0);

        assert_eq!(pte_address(level0), physical_address);
    }
}
