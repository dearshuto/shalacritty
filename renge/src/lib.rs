mod cancellation_token;
mod service_runner;

pub use cancellation_token::{CancelRequest, CancellationToken};
pub use service_runner::{Service, ServiceRunner};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
