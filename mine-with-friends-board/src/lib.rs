mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

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
