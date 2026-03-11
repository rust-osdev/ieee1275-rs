// Copyright 2021 Alberto Ruiz <aruiz@redhat.com>
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::CStr;
use core::mem::size_of;
use core::ptr;

const OF_SIZE_ERR: usize = usize::MAX;

extern "C" fn fallback_entry(_args: *mut Args) -> usize {
    OF_SIZE_ERR
}

pub mod services {
    use core::ffi::c_char;

    use crate::{IHandle, PHandle};
    /// Header for Service Arguments
    #[repr(C)]
    pub struct Args {
        pub service: *const c_char,
        pub nargs: usize,
        pub nret: usize,
    }

    #[repr(C)]
    pub struct WriteArgs {
        pub args: Args,
        pub stdout: *const IHandle,
        pub msg: *const u8,
        pub len: usize,
        pub ret: usize,
    }

    #[repr(C)]
    pub struct FindDeviceArgs {
        pub args: Args,
        pub device: *mut u8,
        pub phandle: *const PHandle,
    }

    #[repr(C)]
    pub struct PropArgs<T> {
        pub args: Args,
        pub phandle: *const PHandle,
        pub prop: *const u8,
        pub buf: *const T,
        pub buflen: usize,
        pub size: usize,
    }

    #[repr(C)]
    pub struct ClaimArgs {
        pub args: Args,
        pub virt: *mut u8,
        pub size: usize,
        pub align: usize,
        pub ret: *mut u8,
    }

    #[repr(C)]
    pub struct ReleaseArgs {
        pub args: Args,
        pub virt: *mut u8,
        pub size: usize,
    }

    #[repr(C)]
    pub struct OpenArgs {
        pub args: Args,
        pub dev: *const c_char,
        pub handle: *const IHandle,
    }

    #[repr(C)]
    pub struct ReadArgs {
        pub args: Args,
        pub handle: *const IHandle,
        pub buffer: *const u8,
        pub size: usize,
        pub actual_size: usize,
    }

    #[repr(C)]
    pub struct CloseArgs {
        pub args: Args,
        pub handle: *const IHandle,
    }

    #[repr(C)]
    pub struct SeekArgs {
        pub args: Args,
        pub handle: *const IHandle,
        pub pos_hi: usize,
        pub pos_low: usize,
        pub status: isize,
    }

    #[repr(C)]
    pub struct CallMethodArgs {
        pub args: Args,
        pub method: *const c_char,
        pub handle: *const IHandle,
    }

    #[repr(C)]
    pub struct BlockSizeArgs {
        pub args: CallMethodArgs,
        pub result: isize,
        pub block_size: isize,
    }

    #[repr(C)]
    pub struct ReadBlocks {
        pub args: CallMethodArgs,
        pub buf: *mut u8,
        pub block_index: usize,
        pub nblocks: usize,
        pub result: usize,
        pub blocks_read: usize,
    }

    #[repr(C)]
    pub struct InterpretArgs {
        pub args: Args,
        pub string: *const c_char,
    }
}

use services::{Args, CallMethodArgs};

#[cfg_attr(not(feature = "no_global_allocator"), global_allocator)]
static mut GLOBAL_PROM: PROM = PROM {
    entry_fn: fallback_entry,
    stdout: ptr::null_mut(),
    stdin: ptr::null_mut(),
    chosen: ptr::null_mut(),
};

#[cfg(not(feature = "no_panic_handler"))]
use core::panic::PanicInfo;

#[cfg(not(feature = "no_panic_handler"))]
#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    unsafe {
        GLOBAL_PROM.exit();
    }
}

/// Opaque type to represent a package handle
#[repr(C)]
pub struct PHandle {}

/// Opaque type to represent a package instance handle
#[repr(C)]
pub struct IHandle {}

/// OF represents an Open Firmware environment
#[derive(Clone, Copy)]
pub struct PROM {
    /// Entry function into the Open Firmware services
    entry_fn: extern "C" fn(*mut Args) -> usize,
    /// Package handle into '/chosen' which holds parameters chosen at runtime
    pub chosen: *const PHandle,
    /// Instance handle into stdout
    pub stdout: *const IHandle,
    /// Instance handle into stdin (for read_stdin)
    pub stdin: *const IHandle,
}

impl PROM {
    /// Creates a new OF instance from a valid entry point
    ///
    /// # Errors
    ///
    /// If it fails on initalization of ```chosen``` and ```stdout``` it will return an error
    pub fn new(entry: extern "C" fn(*mut Args) -> usize) -> Result<Self, &'static str> {
        let mut ret = PROM {
            entry_fn: entry,
            chosen: ptr::null_mut(),
            stdout: ptr::null_mut(),
            stdin: ptr::null_mut(),
        };

        ret.init()?;
        Ok(ret)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        let chosen = self.find_device(c"/chosen")?;
        let mut stdout: *const IHandle = ptr::null_mut();
        let _ = self.get_property(
            chosen,
            c"stdout",
            &mut stdout as *mut *const IHandle,
            core::mem::size_of::<*const IHandle>(),
        )?;
        let mut stdin: *const IHandle = ptr::null_mut();
        let _ = self.get_property(
            chosen,
            c"stdin",
            &mut stdin as *mut *const IHandle,
            core::mem::size_of::<*const IHandle>(),
        )?;

        self.stdout = stdout;
        self.stdin = stdin;
        self.chosen = chosen;
        Ok(())
    }

    /// Exits the client program back into Open Firmware
    pub fn exit(&self) -> ! {
        let mut args = Args {
            service: c"exit".as_ptr(),
            nargs: 1,
            nret: 0,
        };

        (self.entry_fn)(&mut args as *mut Args);
        loop {}
    }

    /// Powers off the system. Does not return.
    pub fn power_off(&self) -> ! {
        let mut args = Args {
            service: c"power-off".as_ptr(),
            nargs: 0,
            nret: 0,
        };

        (self.entry_fn)(&mut args as *mut Args);
        loop {}
    }

    /// Writes raw bytes to stdout (Open Firmware write client service).
    ///
    /// Used by backends that need to send arbitrary bytes (e.g. ANSI sequences plus
    /// UTF-8 or other encodings) without going through a string.
    pub fn write_stdout_bytes(&self, bytes: &[u8]) -> Result<(), &'static str> {
        if self.stdout.is_null() {
            return Err("stdout is not present");
        }
        if bytes.is_empty() {
            return Ok(());
        }

        let mut args = services::WriteArgs {
            args: Args {
                service: c"write".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            stdout: self.stdout,
            msg: bytes.as_ptr(),
            len: bytes.len(),
            ret: 0,
        };

        (self.entry_fn)(&mut args.args as *mut Args);

        match args.ret {
            OF_SIZE_ERR => Err("Error writing stdout"),
            _ => Ok(()),
        }
    }

    /// Writes a string into stdout (implemented on top of [`write_stdout_bytes`](PROM::write_stdout_bytes)).
    pub fn write_stdout(&self, msg: &str) -> Result<(), &'static str> {
        self.write_stdout_bytes(msg.as_bytes())
    }

    /// Writes a str into stdout and ends with a newline
    pub fn write_line(&self, msg: &str) {
        let _ = self.write_stdout(msg);
        let _ = self.write_stdout("\n\r");
    }

    /// Finds a device from a null terminated string
    pub fn find_device(&self, name: &CStr) -> Result<*const PHandle, &'static str> {
        let mut args = services::FindDeviceArgs {
            args: Args {
                service: c"finddevice".as_ptr(),
                nargs: 1,
                nret: 1,
            },
            device: name.as_ptr() as *mut u8,
            phandle: ptr::null_mut(),
        };

        match (self.entry_fn)(&mut args.args as *mut Args) {
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
        phandle: *const PHandle,
        prop: &CStr,
        buf: *mut T,
        buflen: usize,
    ) -> Result<usize, &'static str> {
        let mut args = services::PropArgs {
            args: Args {
                service: c"getprop".as_ptr(),
                nargs: 4,
                nret: 1,
            },
            phandle: phandle,
            prop: prop.as_ptr() as *mut u8,
            buf: buf,
            buflen: buflen,
            size: 0,
        };

        match (self.entry_fn)(&mut args.args as *mut Args) {
            OF_SIZE_ERR => Err("Could not retreive property"),
            _ => Ok(args.size),
        }
    }

    /// Read a cell-sized integer property (e.g. `ibm,secure-boot`). Per the spec, a cell is
    /// pointer-sized; the property must be that many bytes, big-endian.
    pub fn get_integer_property(
        &self,
        phandle: *const PHandle,
        prop: &CStr,
    ) -> Result<isize, &'static str> {
        let mut buf = [0u8; size_of::<usize>()];
        let size = self.get_property(
            phandle,
            prop,
            buf.as_mut_ptr(),
            buf.len(),
        )?;
        if size != size_of::<usize>() {
            return Err("property size is not one cell");
        }
        Ok(usize::from_be_bytes(buf) as isize)
    }

    /// Read bytes from stdin. Returns number of bytes read, or error if stdin is not present.
    pub fn read_stdin(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.stdin.is_null() {
            return Err("stdin is not present");
        }
        self.read(self.stdin, buf.as_mut_ptr(), buf.len())
    }

    /// Allocate heap memory
    ///
    /// # Arguments
    ///
    /// ```size```: The amount of bytes to be allocated
    /// ```align```: The byte alignment boundary, must be graeter than 0
    pub fn claim(&self, size: usize, align: usize) -> Result<*mut u8, &'static str> {
        if align == 0 {
            return Err("Could not allocate memory with alignment '0'");
        }

        let mut args = services::ClaimArgs {
            args: Args {
                service: c"claim".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            virt: ptr::null_mut(),
            size,
            align,
            ret: ptr::null_mut(),
        };

        match (self.entry_fn)(&mut args.args as *mut Args) {
            OF_SIZE_ERR => Err("Could not allocate memory"),
            _ => Ok(args.ret),
        }
    }

    /// Release allocated heap memory by the ```claim``` method
    pub fn release(&self, virt: *mut u8, size: usize) {
        let mut args = services::ReleaseArgs {
            args: Args {
                service: c"release".as_ptr(),
                nargs: 2,
                nret: 0,
            },
            virt,
            size,
        };

        let _ = (self.entry_fn)(&mut args.args as *mut Args);
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
    pub fn open(&self, dev_spec: &CStr) -> Result<*const IHandle, &'static str> {
        let mut args = services::OpenArgs {
            args: Args {
                service: c"open".as_ptr(),
                nargs: 1,
                nret: 1,
            },
            dev: dev_spec.as_ptr(),
            handle: ptr::null(),
        };

        let _ = (self.entry_fn)(&mut args.args as *mut Args);

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
        handle: *const IHandle,
        buffer: *mut u8,
        size: usize,
    ) -> Result<usize, &'static str> {
        let mut args = services::ReadArgs {
            args: Args {
                service: c"read".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            handle,
            buffer,
            size,
            actual_size: 0,
        };

        let _ = (self.entry_fn)(&mut args.args as *mut Args);

        match args.actual_size {
            OF_SIZE_ERR => Err("Could not read device"),
            _ => Ok(args.actual_size),
        }
    }

    pub fn close(&self, handle: *const IHandle) -> Result<(), &'static str> {
        let mut args = services::CloseArgs {
            args: Args {
                service: c"close".as_ptr(),
                nargs: 1,
                nret: 0,
            },
            handle,
        };

        match (self.entry_fn)(&mut args.args as *mut Args) {
            OF_SIZE_ERR => Err("Could not close device"),
            _ => Ok(()),
        }
    }

    pub fn seek(&self, handle: *const IHandle, pos: usize) -> Result<(), &'static str> {
        let mut args = services::SeekArgs {
            args: Args {
                service: c"seek".as_ptr(),
                nargs: 3,
                nret: 1,
            },
            handle,
            pos_hi: 0,
            pos_low: pos,
            status: 0,
        };

        match (self.entry_fn)(&mut args.args as *mut Args) {
            OF_SIZE_ERR => Err("Could not seek device"),
            _ => {
                if args.status == -1 {
                    Err("seek not implemented for this device")
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn get_block_size(&self, block_device: *const IHandle) -> Result<isize, &'static str> {
        let mut args = services::BlockSizeArgs {
            args: CallMethodArgs {
                args: Args {
                    service: c"call-method".as_ptr(),
                    nargs: 2,
                    nret: 2,
                },
                method: c"block-size".as_ptr(),
                handle: block_device,
            },
            result: 0,
            block_size: 0,
        };

        match (self.entry_fn)(&mut args.args.args as *mut Args) {
            OF_SIZE_ERR => Err("Could not get block size for volue device"),
            _ => match args.result {
                0 => Ok(args.block_size),
                _ => Err("Error trying to retrieve block size"),
            },
        }
    }

    /// Execute a Forth command string via the Open Firmware interpret client service (IEEE 1275 §6.3.2.6).
    ///
    /// IN: `[string] cmd`, `stack-arg1`, …, `stack-argP`
    /// OUT: `catch-result`, `stack-result1`, …, `stack-resultQ`
    pub fn interpret(
        &self,
        cmd: &CStr,
        cmd_args: &[isize],
        stack_results: &mut [isize],
    ) -> Result<isize, &'static str> {
        // The usize buffer contains: Interpret Arguments (stack args) + Catch result + Stack results
        let mut buffer = alloc::vec![0usize; (size_of::<Args>() / size_of::<usize>())
            + (cmd_args.len() + 1 + stack_results.len())];
        let mut args = services::InterpretArgs {
            args: Args {
                service: c"interpret".as_ptr(),
                nargs: 1 + cmd_args.len(), // cmd + cmd_args
                nret: 1 + stack_results.len(), // catch result + stack results
            },
            string: cmd.as_ptr(),
        };

        unsafe {
            buffer.as_mut_ptr().copy_from_nonoverlapping(
                (&args as *const services::InterpretArgs) as *const usize,
                size_of::<Args>() / size_of::<usize>(),
            );
        }

        match (self.entry_fn)(&mut args.args as *mut Args) {
            OF_SIZE_ERR => Err("Error interpreting command"),
            _ => unsafe {
                let ret = buffer
                    .as_mut_ptr()
                    .add(size_of::<Args>() / size_of::<usize>() + cmd_args.len())
                    as *mut usize;
                let catch_result = *ret;
                let stack_results_ptr = ret.add(1);
                stack_results.as_mut_ptr().copy_from_nonoverlapping(
                    stack_results_ptr as *const isize,
                    stack_results.len(),
                );
                Ok(catch_result as isize)
            },
        }
    }

    /// Terminal size (columns, rows) via interpret "#columns #lines ". Fallback (80, 24) if interpret fails.
    pub fn terminal_size(&self) -> (u16, u16) {
        let mut out = [0isize; 2];
        if self
            .interpret(c"#columns #lines ", &[], &mut out)
            .is_ok()
        {
            let cols = out[0].clamp(0, u16::MAX as isize) as u16;
            let rows = out[1].clamp(0, u16::MAX as isize) as u16;
            if cols > 0 && rows > 0 {
                return (cols, rows);
            }
        }
        (80, 24)
    }

    /// Current cursor (column, row) via interpret "column# line# ". Returns (0, 0) if interpret fails.
    pub fn get_cursor_position(&self) -> (u16, u16) {
        let mut out = [0isize; 2];
        if self
            .interpret(c"column# line# ", &[], &mut out)
            .is_ok()
        {
            return (
                out[0].clamp(0, u16::MAX as isize) as u16,
                out[1].clamp(0, u16::MAX as isize) as u16,
            );
        }
        (0, 0)
    }

    /// Frame-buffer init via interpret "reset-screen " (call at menu startup).
    /// On frame-buffer consoles this initializes the display; on serial, interpret often
    /// fails or is a no-op. We ignore the result so serial output is not treated as an error.
    pub fn reset_screen(&self) -> Result<(), &'static str> {
        let _ = self.interpret(c"reset-screen ", &[], &mut []);
        Ok(())
    }

    /*pub fn read_blocks(
        &self,
        handle: *const IHandle,
        buf: &mut [u8],
        block_index: usize,
        nblocks: usize,
    ) -> Result<usize, &'static str> {
        let mut args = services::ReadBlocks {
            args: services::CallMethodArgs {
                args: services::Args {
                    service: b"call-method\0".as_ptr(),
                    nargs: 2 + 3,
                    nret: 1 + 1,
                },
                method: b"read-blocks\0".as_ptr(),
                handle,
            },
            buf: buf.as_mut_ptr(),
            block_index,
            nblocks,
            result: usize::MAX,
            blocks_read: 0,
        };

        match (self.entry_fn)(&mut args.args.args as *mut Args) {
            OF_SIZE_ERR => Err("Error reading blocks from volue device"),
            _ => match args.result {
                0 => Ok(args.blocks_read),
                _ => Err("Could not read block from volue device"),
            },
        }
    }*/
}

unsafe impl GlobalAlloc for PROM {
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

/// This function intializes the Open Firmware environment object globally
/// it has to be called before any other API calls are used. Otherwise the
/// default panic and allocation handlers will fail
pub fn prom_init(entry: extern "C" fn(*mut Args) -> usize) -> PROM {
    let prom = match PROM::new(entry) {
        Ok(prom) => prom,
        Err(_) => {
            let mut args = Args {
                service: c"exit".as_ptr(),
                nargs: 0,
                nret: 0,
            };
            let _ = entry(&mut args as *mut Args);
            loop {}
        }
    };

    // WARNING: DO NOT USE alloc:: before this point
    unsafe {
        GLOBAL_PROM = prom;
    };

    prom
}
