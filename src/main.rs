#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::borrow::{Borrow, BorrowMut};
use core::cell::UnsafeCell;
use core::ops::{Add, Deref, DerefMut};
use core::panic::PanicInfo;
use core::ptr;
use core::ptr::write;

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

    unsafe fn free_cell(&self, at: *mut u8) {
        // Note that at is a pointer to the first byte of USED memory, not the start of the cell
        let mut alloc_bit_offset: usize = 1;
        while at.sub(alloc_bit_offset).read() != 1 {
            alloc_bit_offset += 1;
        }
        at.sub(alloc_bit_offset).write(0);
        // TODO: Find adjacent free cells to link/merge
    }

    unsafe fn setup(&self) {
        // Allocate all memory as a single cell
        self.format_cell(HEAP.get() as *mut u8, HEAP_SIZE, false, 0, 0);
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
