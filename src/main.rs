#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::panic::PanicInfo;
mod freelistalloc;


unsafe fn _start() {
    freelistalloc::ALLOCATOR.setup();
}


#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}
