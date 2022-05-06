extern crate ieee1275_rs;
#[cfg(test)]
mod tests {
    use ieee1275_rs::ServiceArgs;
    use ieee1275_rs::PROM;

    struct MOCK_PROM {

    }

    extern "C" fn mock_entry (_args: *mut ServiceArgs) -> usize {
        0
    }

    #[test]
    fn find_device () {
        let PROM = PROM::new(mock_entry);
        PROM.find_device("/chosen");
    }
}