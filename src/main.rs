#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[repr(C)]
struct ServiceArgs {
    service: *const u8,
    nargs: i32,
    nret: i32,
}

#[no_mangle]
#[link_section=".text"]
extern "C" fn _start(_r3: u32, _r4: u32, entry: extern "C" fn (*mut ServiceArgs) -> isize) -> isize {

    #[repr(C)]
    struct GetPropArgs {
        args: ServiceArgs,
        argv: [isize;4],
        retv: [isize;1]
    }

    let mut get_prop_args = GetPropArgs {
        args: ServiceArgs {
            service: "getprop".as_ptr(),
            nargs: 4,
            nret: 1
        },
        argv: [0,0,0,0],
        retv: [0]
    };

    let _ = entry (&mut get_prop_args.args);

    loop {}
}
