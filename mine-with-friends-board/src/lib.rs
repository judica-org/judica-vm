mod callbacks;
pub mod entity;
pub mod tokens;
pub mod game;
pub mod nft;
pub mod sanitize;

pub struct Verified<D> {
    d: D,
    sequence: u64,
    sig: String,
    from: entity::EntityID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
