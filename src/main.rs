#![no_std]
#![no_main]

use core::panic::PanicInfo;
mod freelistalloc;


#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}
