// Copyright 2021 Alberto Ruiz <aruiz@redhat.com>
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

extern crate ieee1275;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, mem::size_of, usize};

    use ieee1275::{services, services::Args, IHandle, PHandle, PROM};

    // Infrastructure to mock an Open Firmware implementation

    const MAX_SERVICE_LENGTH: usize = 100; // We use this threshold to check if non null terminated strings are passed
    const MAX_DEVICE_LENGTH: usize = 500; // We use this threshold to check if non null terminated strings are passed

    const CHOSEN_PHANDLE: usize = 0xdeadbeef;
    const STDOUT_IHANDLE: usize = 0xdecafbad;
    const DISK_IHANDLE: usize = 0xfeedd15c;

    struct MockProm {
        stdout: String,
        stdout_ihandle: usize,
        chosen_phandle: usize,
    }

    struct Heap {
        heap: Option<HashMap<*mut u8, Vec<u8>>>,
    }

    static mut MOCK: MockProm = MockProm {
        stdout: String::new(),
        stdout_ihandle: STDOUT_IHANDLE,
        chosen_phandle: CHOSEN_PHANDLE,
    };
    static mut HEAP: Heap = Heap { heap: None };

    fn cast_args<T>(args: *mut Args) -> &'static mut T {
        unsafe { &mut *(args as *mut T) }
    }

    impl MockProm {
        fn finddevice(&self, args: *mut Args) -> usize {
            let args = cast_args::<services::FindDeviceArgs>(args);
            let device = unsafe { std::slice::from_raw_parts(args.device, MAX_DEVICE_LENGTH) };

            assert_eq!(args.args.nargs, 1);
            assert_eq!(args.args.nret, 1);

            if device.starts_with(b"/chosen\0") {
                (*args).phandle = self.chosen_phandle as *const PHandle;
                size_of::<usize>()
            } else {
                usize::MAX
            }
        }

        fn getprop(&self, args: *mut Args) -> usize {
            let args = cast_args::<services::PropArgs<u8>>(args);
            let prop = unsafe { std::slice::from_raw_parts(args.prop, MAX_SERVICE_LENGTH) };

            assert_eq!(args.args.nargs, 4);
            assert_eq!(args.args.nret, 1);

            if prop.starts_with(b"stdout\0") {
                assert!(args.buflen >= size_of::<usize>());
                let stdout_address: &mut usize = unsafe { &mut *(args.buf as *mut usize) };
                *stdout_address = self.stdout_ihandle;
                args.size = size_of::<usize>();
                args.size
            } else {
                args.size = usize::MAX;
                usize::MAX
            }
        }

        fn write(&self, args: *mut Args) -> usize {
            let args = cast_args::<services::WriteArgs>(args);
            let mock_ref = unsafe { &mut MOCK };

            assert_eq!(args.args.nargs, 3);
            assert_eq!(args.args.nret, 1);

            if format!("{:p}", args.stdout) == "0xdecafbad" {
                let msg: &[u8] = unsafe { std::slice::from_raw_parts(args.msg, args.len) };
                let mut c: usize = 0;
                for i in msg {
                    mock_ref.stdout.push(*i as char);
                    c += 1;
                }
                c
            } else {
                usize::MAX
            }
        }

        fn claim(&self, args: *mut Args) -> usize {
            let args = cast_args::<services::ClaimArgs>(args);
            let heap_ref = unsafe { &mut HEAP };

            if heap_ref.heap.is_none() {
                heap_ref.heap = Some(HashMap::new());
            }

            if args.size == usize::MAX {
                args.ret = unsafe { std::mem::transmute(usize::MAX) };
                return usize::MAX;
            }

            let heap = heap_ref.heap.as_mut().unwrap();
            let mut array = vec![0 as u8; args.size];
            args.ret = array.as_mut_ptr();
            heap.insert(args.ret, array);
            unsafe { std::mem::transmute(args.ret) }
        }

        fn release(&self, args: *mut Args) -> usize {
            let args = cast_args::<services::ReleaseArgs>(args);
            let heap_ref = unsafe { &mut HEAP };

            if heap_ref.heap.is_none() {
                return 0;
            }

            let heap = heap_ref.heap.as_mut().unwrap();
            let _ = heap.remove(&args.virt);

            0
        }

        fn open(&self, args: *mut Args) -> usize {
            let mut args = cast_args::<services::OpenArgs>(args);
            let device = unsafe { std::slice::from_raw_parts(args.dev, MAX_DEVICE_LENGTH) };

            if device.starts_with(b"disk\0") {
                args.handle = DISK_IHANDLE as *const IHandle;
                0
            } else {
                usize::MAX
            }
        }

        fn read(&self, args: *mut Args) -> usize {
            let _args = cast_args::<services::ReadArgs>(args);
            0
        }

        fn close(&self, args: *mut Args) -> usize {
            let _args = cast_args::<services::CloseArgs>(args);
            0
        }

        fn call_method(&self, args: *mut Args) -> usize {
            let cm_args = cast_args::<services::CallMethodArgs>(args);
            let method = unsafe { std::slice::from_raw_parts(cm_args.method, MAX_DEVICE_LENGTH) };

            if method.starts_with(b"block-size")
                && (cm_args.handle == DISK_IHANDLE as *const IHandle)
            {
                let mut bs_args = cast_args::<services::BlockSizeArgs>(args);
                bs_args.result = 0;
                bs_args.block_size = 512;
                0
            } else {
                usize::MAX
            }
        }
    }

    extern "C" fn mock_entry(args: *mut Args) -> usize {
        let service_args = unsafe { &mut (*args) };
        let service =
            unsafe { std::slice::from_raw_parts(service_args.service, MAX_DEVICE_LENGTH) };

        let mock_ref = unsafe { &mut MOCK };

        if service.starts_with(b"finddevice\0") {
            mock_ref.finddevice(args)
        } else if service.starts_with(b"getprop\0") {
            mock_ref.getprop(args)
        } else if service.starts_with(b"write\0") {
            mock_ref.write(args)
        } else if service.starts_with(b"claim\0") {
            mock_ref.claim(args)
        } else if service.starts_with(b"release\0") {
            mock_ref.release(args)
        } else if service.starts_with(b"open\0") {
            mock_ref.open(args)
        } else if service.starts_with(b"read\0") {
            mock_ref.read(args)
        } else if service.starts_with(b"close\0") {
            mock_ref.close(args)
        } else if service.starts_with(b"call-method\0") {
            mock_ref.call_method(args)
        } else {
            println!("Service not implemented in Mock PROM");
            usize::MAX
        }
    }

    // Tests

    #[test]
    fn prom_new() {
        let prom = PROM::new(mock_entry).unwrap();
        assert_eq!(format!("{:p}", prom.chosen), "0xdeadbeef");
        assert_eq!(format!("{:p}", prom.stdout), "0xdecafbad");

        //TODO: We need to find  how to compare function pointers
    }

    #[test]
    fn write_stdout() {
        let mock_ref = unsafe { &mut MOCK };
        let prom = PROM::new(mock_entry).unwrap();
        prom.write_line("one two three");
        assert_eq!(mock_ref.stdout, "one two three\n\r");
    }

    #[test]
    fn claim_release() {
        let prom = PROM::new(mock_entry).unwrap();
        let heap = unsafe { &mut HEAP };

        const ALLOC_LENGHT: usize = 4;

        let ret = prom.claim(ALLOC_LENGHT, 1);
        assert!(ret.is_ok());
        let buffer_ptr = ret.unwrap();

        // When we call prom.claim() the Heap gets created if it doesn't exists so this is always safe
        let heap = heap.heap.as_ref().unwrap();
        let memchunk = heap.get(&buffer_ptr);
        assert!(memchunk.is_some(), "Heap did not find returned address");
        let memchunk = memchunk.unwrap();

        let buffer = unsafe { std::slice::from_raw_parts_mut(buffer_ptr, ALLOC_LENGHT) };
        buffer[0] = 1 as u8;
        buffer[1] = 2 as u8;
        buffer[2] = 3 as u8;
        buffer[3] = 4 as u8;

        assert_eq!(
            memchunk as &[u8], buffer,
            "Allocated memory did not point to the same area {:#?}",
            heap
        );

        prom.release(buffer_ptr, ALLOC_LENGHT);
        assert!(
            heap.get(&buffer_ptr).is_none(),
            "Heap did not get empty after prom.release() {:#?}",
            heap
        );
    }

    #[test]
    fn block_size() {
        let prom = PROM::new(mock_entry).unwrap();

        let disk = prom.open("disk\0").unwrap();
        assert_eq!(disk, DISK_IHANDLE as *const IHandle);
        let dsize = prom.get_block_size(disk).unwrap();
        assert_eq!(dsize, 512);
    }

    // TODO
    #[test]
    fn open() {
        let prom = PROM::new(mock_entry).unwrap();

        let disk = prom.open("disk\0").unwrap();
        assert_eq!(disk, DISK_IHANDLE as *const IHandle);
    }

    #[test]
    fn read() {}

    #[test]
    fn close() {}
}
