//! Handling of Objective-C objects.
//!
//! Note that classes and metaclasses are objects too!
//!
//! Resources:
//! - [Apple's documentation of `id`](https://developer.apple.com/documentation/objectivec/id)
//!   (which for some reason omits that `id` is a pointer type)
//!
//! To make things easier for the host code, our implementation will maintain
//! two linked representations of an object: an [objc_object] struct allocated
//! in guest memory, which needs to maintain the same ABI that Apple's runtime
//! does, and a [HostObject] trait object allocated in host memory, which can be
//! used for any data that only our host code needs to access. As a bonus we get
//! some resilience against guest memory corruption.

use super::Class;
use crate::mem::{Mem, MutPtr, Ptr, SafeRead};
use std::any::Any;

/// Memory layout of a minimal Objective-C object. See [id].
///
/// The name comes from `objc_object` in Apple's runtime.
#[repr(C, packed)]
pub struct objc_object {
    /// In life, sometimes we must ask ourselves... what is existence?
    /// What is the meaning in love and suffering? What is it that drives us to
    /// know? What is the joy in longing for absolutes in a universe abundant
    /// in beautiful subjectivity?
    ///
    /// The `isa` pointer cannot answer these questions.
    ///
    /// But it does tell you what class an object belongs to.
    isa: Class,
}
impl SafeRead for objc_object {}

/// Generic pointer to an Objective-C object (including classes or metaclasses).
///
/// The name is standard Objective-C.
#[allow(non_camel_case_types)]
pub type id = MutPtr<objc_object>;

/// Null pointer for Objective-C objects.
///
/// The name is standard Objective-C.
#[allow(non_upper_case_globals)]
pub const nil: id = Ptr::null();

/// Type for host objects.
pub trait HostObject: Any + 'static {}

/// Trait wrapping [HostObject] with a blanket implementation to make
/// downcasting work. Don't implement it yourself.
///
/// This is a workaround for it not being possible to directly cast
/// `&'a dyn HostObject` to `&'a dyn Any`.
pub trait AnyHostObject {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static);
}
impl<T: HostObject> AnyHostObject for T {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static) {
        self
    }
}

/// Empty host object used by `[NSObject alloc]`.
pub struct TrivialHostObject;
impl HostObject for TrivialHostObject {}

impl super::ObjC {
    /// Read the all-important `isa`.
    pub(super) fn read_isa(object: id, mem: &Mem) -> Class {
        mem.read(object).isa
    }
    /// Write the all-important `isa`.
    pub(super) fn write_isa(object: id, isa: Class, mem: &mut Mem) {
        mem.write(object.cast(), isa)
    }

    /// Allocate a (guest) object (like `[NSObject alloc]`) and associate it
    /// with its host object.
    pub fn alloc_object(
        &mut self,
        isa: Class,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
    ) -> id {
        let guest_object = objc_object { isa };
        let ptr = mem.alloc_and_write(guest_object);
        self.objects.insert(ptr, host_object);
        ptr
    }
}
