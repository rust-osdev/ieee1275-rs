extern crate ieee1275_rs;
#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use ieee1275_rs::{PROM,services::Args, services, PHandle, IHandle};

    const MAX_SERVICE_LENGTH: usize = 100; // We use this threshold to check if non null terminated strings are passed
    const MAX_DEVICE_LENGTH: usize = 500; // We use this threshold to check if non null terminated strings are passed

    const CHOSEN_PHANDLE: usize = 0xdeadbeef;
    const STDOUT_IHANDLE: usize = 0xdecafbad;

    struct MockProm {
        stdout: String,
        stdout_ihandle: usize,
        chosen_phandle: usize,
    }

    impl MockProm {
        fn new() -> Self {
            MockProm {
                stdout: String::new(),
                chosen_phandle: CHOSEN_PHANDLE,
                stdout_ihandle: STDOUT_IHANDLE,
            }
        }

        fn finddevice (&self, args: *mut Args) -> usize {
            let args = unsafe { &mut *(args as *mut services::FindDeviceArgs) };
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
            let args = unsafe { &mut *(args as *mut services::PropArgs<u8>) };
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
    }

    extern "C" fn mock_entry(args: *mut Args) -> usize {
        let mock = MockProm::new();

        let service_args = unsafe { &mut (*args) };
        let service = unsafe { std::slice::from_raw_parts(service_args.service, MAX_DEVICE_LENGTH) };
        
        if service.starts_with(b"finddevice\0") {
            mock.finddevice(args)
        } else if service.starts_with(b"getprop\0") {
            mock.getprop(args)
        } else {
          usize::MAX
        }
    }

    #[test]
    fn init() {
        let prom = PROM::new(mock_entry).unwrap();
        assert_eq!(format!("{:p}", prom.chosen), "0xdeadbeef");
    }
}