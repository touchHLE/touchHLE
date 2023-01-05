//! POSIX Threads implementation.

#![allow(non_camel_case_types)]

pub mod key;
pub mod once;

#[derive(Default)]
pub struct State {
    key: key::State,
}
