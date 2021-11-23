#![feature(try_blocks)]

pub mod common;
pub mod modules;
pub mod schemas;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
