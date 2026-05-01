// `picoquic/picoquic.h` is the kitchen-sink public header; mirroring
// the C tree puts its translation at `picoquic/picoquic.rs`, which
// trips clippy's `module_inception` lint.  Allowed crate-wide.
#![allow(clippy::module_inception)]

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

pub mod picoquic;
