#![cfg_attr(not(test), no_std)]

mod dcp;

mod field {
    pub type SmallField = usize;
    pub type Field = ::core::ops::Range<usize>;
    pub type Rest = ::core::ops::RangeFrom<usize>;
}

pub use dcp::*;
