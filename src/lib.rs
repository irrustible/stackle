#![cfg_attr(not(feature="std"), no_std)]
#![cfg_attr(feature="nightly", feature(naked_functions))]

#[cfg(all(feature="alloc", not(feature="std")))]
extern crate alloc;

pub mod stack;
pub mod switch;
