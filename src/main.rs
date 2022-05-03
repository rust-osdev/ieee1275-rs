#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

use alloc::string::String;
use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::ptr;

extern "C" fn fallback_entry(_args: *mut ServiceArgs) -> isize {
    -1
}

#[global_allocator]
static mut GLOBAL_OF: OF = OF {
    entry_fn: fallback_entry,
    stdout: ptr::null_mut(),
    chosen: ptr::null_mut(),
};

const BUFSIZE: usize = 10000;

extern crate alloc;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    unsafe {
        GLOBAL_OF.exit();
    }
}

#[repr(C)]
struct ServiceArgs {
    service: *const u8,
    nargs: usize,
    nret: usize,
}
#[repr(C)]
struct OFpHandle {}

#[repr(C)]
struct OFiHandle {}

#[derive(Clone, Copy)]
struct OF {
    entry_fn: extern "C" fn(*mut ServiceArgs) -> isize,
    pub chosen: *const OFpHandle,
    pub stdout: *const OFiHandle,
}

impl OF {
    fn new(entry: extern "C" fn(*mut ServiceArgs) -> isize) -> Result<Self, &'static str> {
        let mut ret = OF {
            entry_fn: entry,
            chosen: ptr::null_mut(),
            stdout: ptr::null_mut(),
        };

        ret.init()?;
        Ok(ret)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        let chosen = self.find_device("/chosen\0")?;
        let mut stdout: *const OFiHandle = ptr::null_mut();
        let _ = self.get_property(
            chosen,
            "stdout\0",
            &mut stdout as *mut *const OFiHandle,
            core::mem::size_of::<*const OFiHandle>() as isize,
        )?;

        self.stdout = stdout;
        self.chosen = chosen;
        Ok(())
    }

    pub fn exit(&self) -> ! {
        let mut args = ServiceArgs {
            service: "exit\0".as_ptr(),
            nargs: 1,
            nret: 0,
        };

        (self.entry_fn)(&mut args as *mut ServiceArgs);
        loop {}
    }

    pub fn write_stdout(&self, msg: &str) -> Result<(), &'static str> {
        #[repr(C)]
        struct MsgArgs {
            args: ServiceArgs,
            stdout: *const OFiHandle,
            msg: *const u8,
            len: isize,
            ret: i32,
        }

        let mut args = MsgArgs {
            args: ServiceArgs {
                service: "write\0".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            stdout: self.stdout,
            msg: msg.as_ptr(),
            len: msg.len() as isize,
            ret: 0,
        };

        (self.entry_fn)(&mut args.args as *mut ServiceArgs);

        match args.ret {
            -1 => Err("Error escribiendo en stdout "),
            _ => Ok(()),
        }
    }

    pub fn find_device(&self, name: &str) -> Result<*const OFpHandle, &'static str> {
        #[repr(C)]
        struct FindDeviceArgs {
            args: ServiceArgs,
            device: *mut u8,
            phandle: *const OFpHandle,
        }

        let mut args = FindDeviceArgs {
            args: ServiceArgs {
                service: "finddevice\0".as_ptr() as *mut u8,
                nargs: 1,
                nret: 1,
            },
            device: name.as_ptr() as *mut u8,
            phandle: ptr::null_mut(),
        };

        match (self.entry_fn)(&mut args.args as *mut ServiceArgs) {
            -1 => Err("Could not retreive property"),
            _ => Ok(args.phandle),
        }
    }

    pub fn get_property<T>(
        &self,
        phandle: *const OFpHandle,
        prop: &str,
        buf: *mut T,
        buflen: isize,
    ) -> Result<isize, &'static str> {
        #[repr(C)]
        struct PropArgs<T> {
            args: ServiceArgs,
            phandle: *const OFpHandle,
            prop: *const u8,
            buf: *const T,
            buflen: isize,
            size: isize,
        }

        let mut args = PropArgs {
            args: ServiceArgs {
                service: "getprop\0".as_ptr(),
                nargs: 4,
                nret: 1,
            },
            phandle: phandle,
            prop: prop.as_ptr() as *mut u8,
            buf: buf,
            buflen: buflen,
            size: 0,
        };

        match (self.entry_fn)(&mut args.args as *mut ServiceArgs) {
            -1 => Err("Could not retreive property"),
            _ => Ok(args.size),
        }
    }

    fn claim(&self, size: usize, align: usize) -> Result<*mut u8, &'static str> {
        #[repr(C)]
        struct ClaimArgs {
            args: ServiceArgs,
            virt: *mut u8,
            size: usize,
            align: usize,
            ret: *mut u8,
        }

        if align == 0 {
            return Err("Could not allocate memory with alignment '0'");
        }

        let mut args = ClaimArgs {
            args: ServiceArgs {
                service: "claim\0".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            virt: ptr::null_mut(),
            size,
            align,
            ret: ptr::null_mut(),
        };

        match (self.entry_fn)(&mut args.args as *mut ServiceArgs) {
            -1 => Err("Could not allocate memory"),
            _ => Ok(args.ret),
        }
    }

    fn release(&self, virt: *mut u8, size: usize) {
        #[repr(C)]
        struct ReleaseArgs {
            args: ServiceArgs,
            virt: *mut u8,
            size: usize,
        }

        let mut args = ReleaseArgs {
            args: ServiceArgs {
                service: "release\0".as_ptr(),
                nargs: 2,
                nret: 0,
            },
            virt,
            size,
        };

        let _ = (self.entry_fn)(&mut args.args as *mut ServiceArgs);
    }

    fn open(&self, dev_spec: &str) -> Result<*const OFiHandle, &'static str> {
        #[repr(C)]
        struct OpenArgs {
            args: ServiceArgs,
            dev: *const u8,
            handle: *const OFiHandle,
        }

        let mut args = OpenArgs {
            args: ServiceArgs {
                service: "open\0".as_ptr(),
                nargs: 1,
                nret: 1,
            },
            dev: dev_spec.as_ptr(),
            handle: ptr::null(),
        };

        let _ = (self.entry_fn)(&mut args.args as *mut ServiceArgs);

        match args.handle.is_null() {
            true => Err("Could not open device"),
            false => Ok(args.handle),
        }
    }

    fn read(
        &self,
        handle: *const OFiHandle,
        buffer: *mut u8,
        size: isize,
    ) -> Result<isize, &'static str> {
        #[repr(C)]
        struct ReadArgs {
            args: ServiceArgs,
            handle: *const OFiHandle,
            buffer: *const u8,
            size: isize,
            actual_size: isize,
        }

        let mut args = ReadArgs {
            args: ServiceArgs {
                service: "read\0".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            handle,
            buffer,
            size,
            actual_size: 0,
        };

        let _ = (self.entry_fn)(&mut args.args as *mut ServiceArgs);

        match args.actual_size {
            -1 => Err("Could not read device"),
            _ => Ok(args.actual_size),
        }
    }
}

unsafe impl GlobalAlloc for OF {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.claim(layout.size(), layout.align()) {
            Ok(ret) => ret,
            Err(msg) => {
                panic!("{}", msg);
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.release(ptr, layout.size());
    }
}

#[no_mangle]
#[link_section = ".text"]
extern "C" fn _start(_r3: u32, _r4: u32, entry: extern "C" fn(*mut ServiceArgs) -> isize) -> isize {
    let of = match OF::new(entry) {
        Ok(of) => of,
        Err(_) => return -1,
    };

    // WARNING: DO NOT USE alloc:: before this point
    unsafe {
        GLOBAL_OF = of;
    };

    let _ = of.write_stdout(String::from("Hello from Rust into Open Firmware\n\r").as_str());

    let mut buf: [u8; BUFSIZE] = [0; BUFSIZE];

    let _size = of
        .get_property(
            of.chosen,
            "bootpath\0",
            &mut buf as *mut u8,
            buf.len() as isize,
        )
        .unwrap();
    let mut dev_path = String::new();
    for c in buf {
        if c == 0 {
            break;
        }
        dev_path.push(c as char);
    }
    dev_path.push_str(":1,\\test\\foo.txt\0");

    let file_handle = match of.open(&dev_path) {
        Err(msg) => {
            let _ = of.write_stdout(msg);
            let _ = of.write_stdout("\n\r");
            of.exit();
        }
        Ok(file_handle) => {
            let _ = of.write_stdout("device open\n\r");
            file_handle
        }
    };

    buf = [0; BUFSIZE];
    let mut content: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    match of.read(file_handle, &mut buf as *mut u8, BUFSIZE as isize) {
        Err(msg) => {
            let _ = of.write_stdout(msg);
            let _ = of.write_stdout("\n\r");
        }
        Ok(read_size) => {
            let limit = read_size as usize;
            content.extend_from_slice(&buf[0..limit]);
        }
    };
    content.push(0);

    let content =
        unsafe { String::from_raw_parts(content.as_mut_ptr(), content.len(), content.len()) };

    let _ = of.write_stdout(&content);

    of.exit()
}
