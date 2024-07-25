#![no_std]
#![no_main]

use ieee1275::prom_init;
use ieee1275::services::Args;

#[no_mangle]
#[link_section = ".text"]
extern "C" fn _start(_r3: u32, _r4: u32, entry: extern "C" fn(*mut Args) -> usize) -> isize {
    let prom = prom_init(entry);
    prom.write_line("Hello world!");
    prom.exit();
}
