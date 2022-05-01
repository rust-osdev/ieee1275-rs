#![no_std]
#![no_main]

use core::panic::PanicInfo;

static mut GLOBAL_OF: Option<OF> = None;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    unsafe { 
        match &GLOBAL_OF {
            None => {},
            Some(of) => {  of.exit(); }
        };
    }
    loop {}
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

#[derive(Clone)]
struct OF {
    entry_fn: extern "C" fn(*mut ServiceArgs) -> isize,
    pub chosen: *mut OFpHandle,
    pub stdout: *mut OFiHandle,
}

impl OF {
    fn new(entry: extern "C" fn(*mut ServiceArgs) -> isize) -> Result<Self, &'static str> {
        let mut ret = OF {
            entry_fn: entry,
            chosen: core::ptr::null_mut(),
            stdout: core::ptr::null_mut(),
        };

        ret.init()?;
        Ok(ret)
    }
    
    fn init(&mut self) -> Result<(), &'static str> {
        let chosen = self.find_device("/chosen\0")?;
        let mut stdout: *mut OFiHandle = core::ptr::null_mut();
        self.get_property(
            chosen,
            "stdout\0",
            &mut stdout as *mut *mut OFiHandle,
            core::mem::size_of::<*mut OFiHandle>() as isize
        )?;

        self.stdout = stdout;
        self.chosen = chosen;
        Ok(())
    }

    pub fn exit(&self) {
        #[repr(C)]
        struct ExitArgs {
            args: ServiceArgs,
            ret: i32
        }

        let mut args = ExitArgs {
            args: ServiceArgs {
                service: "exit\0".as_ptr(),
                nargs: 1,
                nret: 1
            },
            ret: 0
        };

        (self.entry_fn)(&mut args.args as *mut ServiceArgs);
    }

    pub fn write_stdout(&self, msg: &'static str) -> Result<(), &'static str>{
        #[repr(C)]
        struct MsgArgs {
            args: ServiceArgs,
            stdout: *mut OFiHandle,
            msg: *const u8,
            len: isize,
            ret: i32
        }

        let mut args = MsgArgs {
            args: ServiceArgs {
                service: "write\0".as_ptr(),
                nargs: 3,
                nret: 1
            },
            stdout: self.stdout,
            msg: msg.as_ptr(),
            len: isize::try_from(msg.len()).unwrap(),
            ret: 0
        };
        
        (self.entry_fn)(&mut args.args as *mut ServiceArgs);

        match args.ret {
            -1 => Err("Error escribiendo en stdout "),
            _ => Ok(()),
        }
    }

    pub fn find_device(
        &self,
        name: &str
    ) -> Result<*mut OFpHandle, &'static str> {
        #[repr(C)]
        struct PropArgs {
            args: ServiceArgs,
            device: *mut u8,
            phandle: *mut OFpHandle,
        }

        let mut args = PropArgs {
            args: ServiceArgs {
                service: "finddevice\0".as_ptr() as *mut u8,
                nargs: 1,
                nret: 1,
            },
            device: name.as_ptr() as *mut u8,
            phandle: core::ptr::null_mut(),
        };

        match (self.entry_fn)(&mut args.args as *mut ServiceArgs) {
            -1 => Err("Could not retreive property"),
            _ => Ok(args.phandle),
        }
    }

    pub fn get_property<T>(
        &self,
        phandle: *mut OFpHandle,
        prop: &str,
        buf: *mut T,
        buflen: isize
    ) -> Result<(), &'static str> {
        #[repr(C)]
        struct PropArgs<T> {
            args: ServiceArgs,
            phandle: *mut OFpHandle,
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
            _ => Ok(()),
        }
    }
}

#[no_mangle]
#[link_section = ".text"]
extern "C" fn _start(_r3: u32, _r4: u32, entry: extern "C" fn(*mut ServiceArgs) -> isize) -> isize {
    let of = match OF::new(entry) {
        Ok(of) => of,
        Err(_) => return -1,
    };

    unsafe {
        GLOBAL_OF = Some(of.clone());
    };

    let _ = of.write_stdout("Hello from Rust into Open Firmware");

    loop {}
}
