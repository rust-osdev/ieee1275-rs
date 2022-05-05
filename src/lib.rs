#![no_std]
#![feature(default_alloc_error_handler)]

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::ptr;

const OF_SIZE_ERR: usize = usize::MAX;

extern "C" fn fallback_entry(_args: *mut ServiceArgs) -> usize {
    OF_SIZE_ERR
}

#[global_allocator]
static mut GLOBAL_OF: OF = OF {
    entry_fn: fallback_entry,
    stdout: ptr::null_mut(),
    chosen: ptr::null_mut(),
};

extern crate alloc;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    unsafe {
        GLOBAL_OF.exit();
    }
}

/// Header for Service Arguments
#[repr(C)]
pub struct ServiceArgs {
    service: *const u8,
    nargs: usize,
    nret: usize,
}

/// Opaque type to represent a package handle
#[repr(C)]
pub struct OFpHandle {}

/// Opaque type to represent a package instance handle
#[repr(C)]
pub struct OFiHandle {}

/// OF represents an Open Firmware environment
#[derive(Clone, Copy)]
pub struct OF {
    /// Entry function into the Open Firmware services
    entry_fn: extern "C" fn(*mut ServiceArgs) -> usize,
    /// Package handle into '/chosen' which holds parameters chosen at runtime
    pub chosen: *const OFpHandle,
    /// Instance handle into stdout
    pub stdout: *const OFiHandle,
}

impl OF {
    /// Creates a new OF instance from a valid entry point
    ///
    /// # Errors
    ///
    /// If it fails on initalization of ```chosen``` and ```stdout``` it will return an error
    pub fn new(entry: extern "C" fn(*mut ServiceArgs) -> usize) -> Result<Self, &'static str> {
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
            core::mem::size_of::<*const OFiHandle>(),
        )?;

        self.stdout = stdout;
        self.chosen = chosen;
        Ok(())
    }

    /// Exits the client program back into Open Firmware
    pub fn exit(&self) -> ! {
        let mut args = ServiceArgs {
            service: "exit\0".as_ptr(),
            nargs: 1,
            nret: 0,
        };

        (self.entry_fn)(&mut args as *mut ServiceArgs);
        loop {}
    }

    /// Writes a string into stdout
    pub fn write_stdout(&self, msg: &str) -> Result<(), &'static str> {
        #[repr(C)]
        struct MsgArgs {
            args: ServiceArgs,
            stdout: *const OFiHandle,
            msg: *const u8,
            len: usize,
            ret: usize,
        }

        if self.stdout.is_null() {
            return Err("stdout is not present");
        }

        let mut args = MsgArgs {
            args: ServiceArgs {
                service: "write\0".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            stdout: self.stdout,
            msg: msg.as_ptr(),
            len: msg.len(),
            ret: 0,
        };

        (self.entry_fn)(&mut args.args as *mut ServiceArgs);

        match args.ret {
            OF_SIZE_ERR => Err("Error writing stdout"),
            _ => Ok(()),
        }
    }

    /// Writes a str into stdout and ends with a newline
    pub fn write_line(&self, msg: &str) {
        let _ = self.write_stdout(msg);
        let _ = self.write_stdout("\n\r");
    }

    /// Finds a device from a null terminated string
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
            OF_SIZE_ERR => Err("Could not retreive property"),
            _ => Ok(args.phandle),
        }
    }

    /// Get property from package
    ///
    /// # Arguments
    ///
    /// ```phandle```: package handle
    /// ```prop```: null terminated property name
    /// ```buf```: pointer to buffer to store the value of the property, note that it has to match the known size of the property
    /// ```buflen```: length of ```buf```
    ///
    /// # Retuns
    ///
    /// The actual amount of bytes written
    pub fn get_property<T>(
        &self,
        phandle: *const OFpHandle,
        prop: &str,
        buf: *mut T,
        buflen: usize,
    ) -> Result<usize, &'static str> {
        #[repr(C)]
        struct PropArgs<T> {
            args: ServiceArgs,
            phandle: *const OFpHandle,
            prop: *const u8,
            buf: *const T,
            buflen: usize,
            size: usize,
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
            OF_SIZE_ERR => Err("Could not retreive property"),
            _ => Ok(args.size),
        }
    }

    /// Allocate heap memory
    ///
    /// # Arguments
    ///
    /// ```size```: The amount of bytes to be allocated
    /// ```align```: The byte alignment boundary, must be graeter than 0
    pub fn claim(&self, size: usize, align: usize) -> Result<*mut u8, &'static str> {
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
            OF_SIZE_ERR => Err("Could not allocate memory"),
            _ => Ok(args.ret),
        }
    }

    /// Release allocated heap memory by the ```claim``` method
    pub fn release(&self, virt: *mut u8, size: usize) {
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

    /// Opens a device from a spec
    ///
    /// # Arguments
    ///
    /// ```dev_spec```: The device specifier, must be a null terminated string
    ///
    /// # Returns
    ///
    /// Pointer to the device's package instance handle on success
    pub fn open(&self, dev_spec: &str) -> Result<*const OFiHandle, &'static str> {
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

    /// Read operation
    ///
    /// # Arguments
    ///
    /// ```handle```: Instance handle
    /// ```buffer```: Output buffer to write the read content
    /// ```size```: Size in bytes of the output buffer
    ///
    /// # Returns
    ///
    /// Number of bytes read into ```buffer```
    pub fn read(
        &self,
        handle: *const OFiHandle,
        buffer: *mut u8,
        size: usize,
    ) -> Result<usize, &'static str> {
        #[repr(C)]
        struct ReadArgs {
            args: ServiceArgs,
            handle: *const OFiHandle,
            buffer: *const u8,
            size: usize,
            actual_size: usize,
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
            OF_SIZE_ERR => Err("Could not read device"),
            _ => Ok(args.actual_size),
        }
    }

    pub fn close(&self, handle: *const OFiHandle) -> Result<(), &'static str> {
        #[repr(C)]
        struct CloseArgs {
            args: ServiceArgs,
            handle: *const OFiHandle,
        }

        let mut args = CloseArgs {
            args: ServiceArgs {
                service: "close\0".as_ptr(),
                nargs: 1,
                nret: 0,
            },
            handle,
        };

        match (self.entry_fn)(&mut args.args as *mut ServiceArgs) {
            OF_SIZE_ERR => Err("Could not close device"),
            _ => Ok(()),
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

pub fn of_init(entry: extern "C" fn(*mut ServiceArgs) -> usize) -> OF {
    let of = match OF::new(entry) {
        Ok(of) => of,
        Err(_) => {
            let mut args = ServiceArgs {
                service: "exit\0".as_ptr(),
                nargs: 0,
                nret: 0,
            };
            let _ = entry(&mut args as *mut ServiceArgs);
            loop {}
        }
    };

    // WARNING: DO NOT USE alloc:: before this point
    unsafe {
        GLOBAL_OF = of;
    };

    of
}