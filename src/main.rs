#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::borrow::{Borrow, BorrowMut};
use core::cell::UnsafeCell;
use core::ops::{Add, Deref, DerefMut};
use core::panic::PanicInfo;
use core::ptr;
use core::ptr::{null_mut, write};

const STACK_SIZE_CONST: usize = 100000;
static STACK_SIZE: usize = STACK_SIZE_CONST;
static mut STACK: [u8; STACK_SIZE_CONST] = [0; STACK_SIZE_CONST];
const HEAP_SIZE_CONST: usize = 2000000;
static HEAP_SIZE: usize = HEAP_SIZE_CONST;
static mut HEAP: UnsafeCell<[u8; HEAP_SIZE_CONST]> = UnsafeCell::new([0; HEAP_SIZE_CONST]);


struct Heap {}

#[global_allocator]
static ALLOCATOR: Heap = Heap {};

impl Heap {

    unsafe fn write_usize32(&self, at: *mut u8, val: usize) {
        at.write(((val & 0xFF000000) >> 24) as u8);
        at.add(1).write(((val & 0x00FF0000) >> 16) as u8);
        at.add(2).write(((val & 0x0000FF00) >> 8) as u8);
        at.add(3).write((val & 0x000000FF) as u8);
    }
    
    unsafe fn read_usize32(&self, at: *const u8) -> usize {
        return ((at.read() as usize) << 24) +
            ((at.add(1).read() as usize) << 16) +
            ((at.add(2).read() as usize) << 8) +
            (at.add(3).read() as usize);
    }

    unsafe fn format_cell(
        &self,
        at: *mut u8,
        size: usize,
        allocated: bool,
        prev_offset: usize,
        next_offset: usize
    ) {
        // Note that at is a pointer to the first byte of the CELL, not used memory
        self.write_usize32(at, size);
        at.add(4).write(allocated as u8);
        self.write_usize32(at.add(5), prev_offset);
        self.write_usize32(at.add(9), next_offset);
    }

    #[inline(always)]
    unsafe fn read_cell_size(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.sub(4));
    }

    #[inline(always)]
    unsafe fn read_cell_prev_offset(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.add(1));
    }

    #[inline(always)]
    unsafe fn read_cell_next_offset(&self, at: *mut u8) -> usize {
        return self.read_usize32(at.add(5));
    }

    #[inline(always)]
    unsafe fn write_cell_size(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.sub(4), val);
    }

    #[inline(always)]
    unsafe fn write_cell_prev_offset(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.add(1), val);
    }

    #[inline(always)]
    unsafe fn write_cell_next_offset(&self, at: *mut u8, val: usize) {
        self.write_usize32(at.add(5), val);
    }

    unsafe fn find_adjacent_free_cell(&self, at: *mut u8) -> *mut u8 {
        let mut heap_ptr: *mut u8 = HEAP.get() as *mut u8;
        let mut next_offset: usize = self.read_usize32(heap_ptr);
        if next_offset == 0 {
            return null_mut();
        }
        heap_ptr = heap_ptr.add(next_offset);
        next_offset = self.read_cell_next_offset(heap_ptr);
        while next_offset > 0 && heap_ptr.add(next_offset - 4) < at {
            heap_ptr = heap_ptr.add(next_offset);
            next_offset = self.read_cell_next_offset(heap_ptr);
        }
        return heap_ptr;
    }

    unsafe fn free_cell(&self, at: *mut u8) {
        // Note that at is a pointer to the first byte of USED memory, not the start of the cell
        let mut alloc_bit_offset: usize = 1;
        while at.sub(alloc_bit_offset).read() != 1 {
            alloc_bit_offset += 1;
        }
        let current_alloc_addr: usize = at as usize - alloc_bit_offset;
        at.sub(alloc_bit_offset).write(0);

        // Find the closest free cell with a lower address
        let prev_cell: *mut u8 = self.find_adjacent_free_cell(at);

        if !prev_cell.is_null() {
            // A cell was found, store the offset to the next cell
            let next_offset: usize = self.read_cell_next_offset(prev_cell);

            // Make the previous cell point to this cell
            self.write_cell_next_offset(
                prev_cell,
                current_alloc_addr - prev_cell as usize
            );

            if next_offset > 0 {
                // If the previous cell was pointing to another cell, that cell must be the
                // closest free cell with a larger address, so update its prev offset
                let next_cell = prev_cell.add(
                    self.read_cell_next_offset(prev_cell)
                );
                self.write_cell_prev_offset(
                    next_cell,
                    next_cell as usize - current_alloc_addr
                );
            }
        }
        else {
            // There is no free cell with a smaller address, so point the initial offset here
            let mut heap_ptr: *mut u8 = HEAP.get() as *mut u8;
            let next_offset: usize = self.read_usize32(heap_ptr);

            // If the initial offset pointed somewhere, update its previous offset
            if next_offset > 0 {
                let next_cell = heap_ptr.add(next_offset);
                self.write_cell_prev_offset(
                    next_cell,
                    next_cell as usize - current_alloc_addr
                );
            }
            self.write_usize32(heap_ptr, current_alloc_addr - heap_ptr as usize);
        }
        // TODO: Try to merge cells
    }

    unsafe fn setup(&self) {
        // Allocate all memory as a single cell
        let mut heap_ptr: *mut u8 = HEAP.get() as *mut u8;
        self.write_usize32(heap_ptr, 5); // Initial offset to first free cell
        self.format_cell(
            heap_ptr.add(4),
            HEAP_SIZE - 17,
            false,
            0,
            0
        ); // Write a cell containing all remaining memory
    }

}

unsafe impl GlobalAlloc for Heap {

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        todo!()
    }

}


#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}
