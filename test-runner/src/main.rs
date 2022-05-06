extern crate ieee1275_rs;

fn main () {

}

#[cfg(test)]
mod tests {
    use ieee1275_rs::ServiceArgs;
    fn entry (_args: *mut ServiceArgs) -> usize {
        0
    }

    #[test]
    fn foobar () {
        entry(std::ptr::null_mut());
    }
}