mod client;
mod detail;
mod server;

pub use client::{Client, Profile};
pub use detail::{ProfileListRequest, ProfileListResponse};
pub use server::{IServerBackend, Server};

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
