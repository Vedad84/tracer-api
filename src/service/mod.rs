pub mod eip1898;
pub mod geth;


pub trait To<T> {
    fn to(self) -> T;
}
