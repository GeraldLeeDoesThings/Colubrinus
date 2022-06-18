#![no_std]
#![no_main]

use core::panic::PanicInfo;
mod freelistalloc;


unsafe fn _start() {
    freelistalloc::ALLOCATOR.setup();
}


#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}
