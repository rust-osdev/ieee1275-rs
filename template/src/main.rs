#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use ieee1275_rs::{ServiceArgs, prom_init};

const BUFSIZE: usize = 10000;


#[no_mangle]
#[link_section = ".text"]
extern "C" fn _start(_r3: u32, _r4: u32, entry: extern "C" fn(*mut ServiceArgs) -> usize) -> isize {
    let prom = prom_init(entry);
    prom.write_line(String::from("Hello from Rust into Open Firmware\n\r").as_str());

    let mut buf: [u8; BUFSIZE] = [0; BUFSIZE];

    let _size = prom
        .get_property(prom.chosen, "bootpath\0", &mut buf as *mut u8, buf.len())
        .unwrap();
    let mut dev_path = String::new();
    for c in buf {
        if c == 0 {
            break;
        }
        dev_path.push(c as char);
    }
    dev_path.push_str(":1,\\loader\\index.lst\0");

    let file_handle = match prom.open(&dev_path) {
        Err(msg) => {
            prom.write_line(msg);
            prom.exit();
        }
        Ok(file_handle) => file_handle,
    };

    buf = [0; BUFSIZE];
    let content = match prom.read(file_handle, &mut buf as *mut u8, BUFSIZE) {
        Err(msg) => {
            prom.write_line(msg);
            prom.exit();
        }
        Ok(read_size) => {
            let mut content: Vec<u8> = Vec::new();
            content.extend_from_slice(&mut buf[0..read_size]);
            unsafe { String::from_raw_parts(buf.as_mut_ptr(), content.len(), content.capacity()) }
        }
    };

    prom.write_line(&content);
    if let Err(msg) = prom.close(file_handle) {
        prom.write_line(msg);
    }

    prom.exit()
}
