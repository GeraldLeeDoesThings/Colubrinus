#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::borrow::{Borrow, BorrowMut};
use core::ops::{Deref, DerefMut};
use core::panic::PanicInfo;
use core::ptr;

const STACK_SIZE_CONST: usize = 100000;
static STACK_SIZE: usize = STACK_SIZE_CONST;
static mut STACK: [u8; STACK_SIZE_CONST] = [0; STACK_SIZE_CONST];
const HEAP_SIZE_CONST: usize = 2000000;
static HEAP_SIZE: usize = HEAP_SIZE_CONST;
static mut HEAP: [u8; HEAP_SIZE_CONST] = [0; HEAP_SIZE_CONST];


struct Heap {}

#[global_allocator]
static ALLOCATOR: Heap = Heap {};

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
