pub mod client;

pub mod realm;
pub mod governance;
pub mod proposal;
pub mod token_owner;

pub mod addin_fixed_weights;
pub mod addin_vesting;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
