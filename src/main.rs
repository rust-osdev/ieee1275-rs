#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[repr(C)]
struct ServiceArgs {
    service: *const char,
    nargs: i32,
    ret: i32,
}

#[no_mangle]
#[link_section=".text"]
extern "C" fn _start(_r3: u32, _r4: u32, _r5: extern "C" fn (*mut ServiceArgs) -> isize) -> isize {

    loop {}
}
