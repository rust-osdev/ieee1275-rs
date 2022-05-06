extern crate ieee1275_rs;
#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use ieee1275_rs::{PHandle, PROM, services::{Args, FindDeviceArgs}, services};

    // Infrastructure to mock an Open Firmware implementation

    const MAX_SERVICE_LENGTH: usize = 100; // We use this threshold to check if non null terminated strings are passed
    const MAX_DEVICE_LENGTH: usize = 500; // We use this threshold to check if non null terminated strings are passed

    const CHOSEN_PHANDLE: usize = 0xdeadbeef;
    const STDOUT_IHANDLE: usize = 0xdecafbad;

    struct MockProm {
        stdout: String,
        stdout_ihandle: usize,
        chosen_phandle: usize,
    }

    static mut MOCK: MockProm = MockProm { stdout: String::new(), stdout_ihandle: STDOUT_IHANDLE, chosen_phandle: CHOSEN_PHANDLE };

    fn cast_args<T> (args: *mut Args) -> &'static mut T {
        unsafe { &mut *(args as *mut T) }
    }

    impl MockProm {
        fn clear(&mut self) {
            self.stdout.clear();
        }

        fn finddevice (&self, args: *mut Args) -> usize {
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

        fn getprop (&self, args: *mut Args) -> usize {
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
                usize::MAX
            }
        }

        fn write (&self, args: *mut Args) -> usize {
            let args = cast_args::<services::WriteArgs>(args);
            let mock_ref = unsafe {&mut MOCK};

            assert_eq!(args.args.nargs, 3);
            assert_eq!(args.args.nret, 1);
            if format!("{:p}", args.stdout) == "0xdecafbad" {
                let msg: &[u8] = unsafe { std::slice::from_raw_parts(args.msg, args.len) };
                let mut c: usize = 0;
                for i in msg {
                    mock_ref.stdout.push(*i as char);
                    c += 1;
                };
                c
            } else {
                usize::MAX
            }
        }

        fn claim (&self, args: *mut Args) -> usize {
            let args = cast_args::<services::ClaimArgs>(args);
            let mock_ref = unsafe {&mut MOCK};
            0
        }

        fn release (&self, args: *mut Args) -> usize {
            let args = cast_args::<services::ReleaseArgs>(args);
            let mock_ref = unsafe {&mut MOCK};
            0
        }

        fn open (&self, args: *mut Args) -> usize {
            let args = cast_args::<services::OpenArgs>(args);
            let mock_ref = unsafe {&mut MOCK};
            0
        }

        fn read (&self, args: *mut Args) -> usize {
            let args = cast_args::<services::ReadArgs>(args);
            let mock_ref = unsafe {&mut MOCK};
            0
        }

        fn close (&self, args: *mut Args) -> usize {
            let mock_ref = unsafe {&mut MOCK};
            0
        }
    }

    extern "C" fn mock_entry(args: *mut Args) -> usize {
        let service_args = unsafe { &mut (*args) };
        let service = unsafe { std::slice::from_raw_parts(service_args.service, MAX_DEVICE_LENGTH) };

        let mock_ref = unsafe {&mut MOCK};
        
        if service.starts_with(b"finddevice\0") {
            mock_ref.finddevice(args)
        } else if service.starts_with(b"getprop\0") {
            mock_ref.getprop(args)
        } else if service.starts_with(b"write\0") {
            mock_ref.write(args)
        } else {
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
        let mock_ref = unsafe {&mut MOCK};
        let prom = PROM::new(mock_entry).unwrap();
        prom.write_line("one two three");
        assert_eq!(mock_ref.stdout, "one two three\n\r");
    }
}