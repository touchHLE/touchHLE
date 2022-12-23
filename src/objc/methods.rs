//! Handling of Objective-C methods.
//!
//! Resources:
//! - [Apple's documentation of `class_addMethod`](https://developer.apple.com/documentation/objectivec/1418901-class_addmethod?language=objc)

use super::{id, SEL};
use crate::abi::{CallFromGuest, GuestArg, GuestRet};
use crate::Environment;

/// Type for any function implementating a method.
///
/// The name is standard Objective-C.
///
/// In our implementation, we have both "host methods" (Rust functions) and
/// "guest methods" (functions in the guest app, TODO), so this has to be an
/// enum. Either way, the function needs to conform to the same ABI: [id] and
/// [SEL] must be its first two parameters.
pub enum IMP {
    Host(&'static dyn HostIMP),
    // TODO: Guest(GuestIMP),
}

/// Type for any host function implementing a method (see also [IMP]).
pub trait HostIMP: CallFromGuest {}

impl<R> HostIMP for fn(&mut Environment, id, SEL) -> R where R: GuestRet {}
impl<R, P1> HostIMP for fn(&mut Environment, id, SEL, P1) -> R
where
    R: GuestRet,
    P1: GuestArg,
{
}
impl<R, P1, P2> HostIMP for fn(&mut Environment, id, SEL, P1, P2) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
{
}

// TODO: pub type GuestIMP = ...;
