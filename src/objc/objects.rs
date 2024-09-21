/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
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
//!
//! See also: [crate::frameworks::foundation::ns_object].

use super::{Class, ClassHostObject};
use crate::mem::{guest_size_of, GuestUSize, Mem, MutPtr, Ptr, SafeRead};
use std::any::Any;
use std::num::NonZeroU32;

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
    pub(super) isa: Class,
}
unsafe impl SafeRead for objc_object {}

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

/// Struct used to track the host object and refcount of every object.
/// Maybe debugging info too eventually?
///
/// If the `refcount` is `None`, that means this object has a static duration
/// and should not be reference-counted, e.g. it is a class.
pub(super) struct HostObjectEntry {
    host_object: Box<dyn AnyHostObject>,
    refcount: Option<NonZeroU32>,
}

/// Type for host objects.
pub trait HostObject: Any + 'static {
    /// Upcast to the superclass's host object type, if any.
    ///
    /// In order to support inheritance, a class's host object may extend its
    /// superclass's host object. For the superclass to be able to access its
    /// data without being aware of the subclass, it needs to be able to upcast
    /// a host object.
    ///
    /// When trying to downcast from [AnyHostObject] to some type fails, you
    /// should upcast and try again, repeatedly if necessary. This ensures that
    /// if the object actually belongs to a subclass of the target type, the
    /// downcast will eventually succeed. If [None] is returned by this method,
    /// there are no more superclasses' host objects in the chain.
    fn as_superclass<'a>(&'a self) -> Option<&'a (dyn AnyHostObject + 'static)> {
        None
    }
    /// Same as [HostObject::as_superclass], but for a mutable reference.
    fn as_superclass_mut<'a>(&'a mut self) -> Option<&'a mut (dyn AnyHostObject + 'static)> {
        None
    }
}

/// Convenience macro for implementing [HostObject] where the host object type
/// extends some superclass's host object type. The superclass's host object
/// must be in a struct member named `superclass`.
///
/// Example usage, if `Bar` extends `Foo`:
///
/// ```ignore
/// struct BarHostObject {
///     superclass: FooHostObject,
///     some_extra_data: id,
/// }
/// impl_HostObject_with_superclass!(BarHostObject);
/// ```
///
/// If the superclass doesn't have a meaningful host object (e.g. `NSObject`
/// uses [TrivialHostObject]), `impl HostObject for FooHostObject {}` suffices.
#[macro_export]
macro_rules! impl_HostObject_with_superclass {
    ( $ty:ty ) => {
        impl $crate::objc::HostObject for $ty {
            fn as_superclass<'a>(
                &'a self,
            ) -> Option<&'a (dyn $crate::objc::AnyHostObject + 'static)> {
                Some(&self.superclass)
            }
            fn as_superclass_mut<'a>(
                &'a mut self,
            ) -> Option<&'a mut (dyn $crate::objc::AnyHostObject + 'static)> {
                Some(&mut self.superclass)
            }
        }
    };
}
pub use crate::impl_HostObject_with_superclass; // #[macro_export] is weird...

/// Trait wrapping [HostObject] with a blanket implementation to make
/// downcasting work. Don't implement it yourself.
///
/// This is a workaround for it not being possible to directly cast
/// `&'a dyn HostObject` to `&'a dyn Any`.
pub trait AnyHostObject: HostObject {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static);
    fn as_any_mut<'a>(&'a mut self) -> &'a mut (dyn Any + 'static);
}
impl<T: HostObject> AnyHostObject for T {
    fn as_any<'a>(&'a self) -> &'a (dyn Any + 'static) {
        self
    }
    fn as_any_mut<'a>(&'a mut self) -> &'a mut (dyn Any + 'static) {
        self
    }
}

/// Empty host object used by `[NSObject alloc]`.
pub struct TrivialHostObject;
impl HostObject for TrivialHostObject {}

impl super::ObjC {
    /// Read the all-important `isa`.
    pub fn read_isa(object: id, mem: &Mem) -> Class {
        mem.read(object).isa
    }

    fn alloc_object_inner(
        &mut self,
        isa: Class,
        instance_size: GuestUSize,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
        refcount: Option<NonZeroU32>,
    ) -> id {
        let guest_object = objc_object { isa };
        // FIXME: Apparently some classes have an instance size of 0?
        //        Figure out what that actually means and remove this hack.
        let instance_size = instance_size.max(guest_size_of::<objc_object>());
        assert!(instance_size >= guest_size_of::<objc_object>());

        let ptr: MutPtr<objc_object> = mem.alloc(instance_size).cast();
        mem.write(ptr, guest_object);
        assert!(!self.objects.contains_key(&ptr));
        self.objects.insert(
            ptr,
            HostObjectEntry {
                host_object,
                refcount,
            },
        );
        ptr
    }

    /// Allocate a reference-counted (guest) object (like `[NSObject alloc]`)
    /// and associate it with its host object.
    ///
    /// `isa` must be a real class, as the instance size will be fetched from
    /// the class.
    pub fn alloc_object(
        &mut self,
        isa: Class,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
    ) -> id {
        let &ClassHostObject { instance_size, .. } = self.borrow(isa);
        self.alloc_object_inner(
            isa,
            instance_size,
            host_object,
            mem,
            Some(NonZeroU32::new(1).unwrap()),
        )
    }

    /// Allocate a static-lifetime (guest) object (for example, a class) and
    /// associate it with its host object.
    ///
    /// It is assumed that the guest object's instance size is 4 (just an `isa`)
    /// like `NSObject`. This means you must not use this function to implement
    /// the `alloc` method of a class that could be the superclass of any class
    /// in the guest app!
    pub fn alloc_static_object(
        &mut self,
        isa: Class,
        host_object: Box<dyn AnyHostObject>,
        mem: &mut Mem,
    ) -> id {
        let size = guest_size_of::<objc_object>();
        self.alloc_object_inner(isa, size, host_object, mem, None)
    }

    /// Associate a host object with an existing static-lifetime (guest) object
    /// (for example, a class).
    pub fn register_static_object(
        &mut self,
        guest_object: id,
        host_object: Box<dyn AnyHostObject>,
    ) {
        assert!(!self.objects.contains_key(&guest_object));
        self.objects.insert(
            guest_object,
            HostObjectEntry {
                host_object,
                refcount: None,
            },
        );
    }

    /// Get a reference to a host object, if the object exists.
    pub fn get_host_object(&self, object: id) -> Option<&dyn AnyHostObject> {
        self.objects.get(&object).map(|entry| &*entry.host_object)
    }

    /// Get a reference to a host object and downcast it. Panics if there is
    /// no such object, or if downcasting fails.
    pub fn borrow<T: AnyHostObject + 'static>(&self, object: id) -> &T {
        let mut host_object: &(dyn AnyHostObject + 'static) =
            &*self.objects.get(&object).unwrap().host_object;
        loop {
            if let Some(res) = host_object.as_any().downcast_ref() {
                return res;
            } else if let Some(next) = host_object.as_superclass() {
                host_object = next;
            } else {
                panic!();
            }
        }
    }

    /// Get a reference to a host object and downcast it. Panics if there is
    /// no such object, or if downcasting fails.
    pub fn borrow_mut<T: AnyHostObject + 'static>(&mut self, object: id) -> &mut T {
        // Rust's borrow checker struggles with loops like this which descend
        // through a data structure with a mutable borrow. The unsafe code is
        // used to bypass the borrow checker.
        type Aho = dyn AnyHostObject + 'static;
        let mut host_object: &mut Aho = &mut *self.objects.get_mut(&object).unwrap().host_object;
        loop {
            if let Some(res) = unsafe { &mut *(host_object as *mut Aho) }
                .as_any_mut()
                .downcast_mut()
            {
                return res;
            } else if let Some(next) = host_object.as_superclass_mut() {
                host_object = next;
            } else {
                panic!();
            }
        }
    }

    /// Getting a refcount of an object.
    /// While Apple's docs advise to not relay on the returned value,
    /// some games (like "Cut the Rope") does call `retainCount`.
    pub fn get_refcount(&mut self, object: id) -> NonZeroU32 {
        let Some(entry) = self.objects.get_mut(&object) else {
            panic!(
                "No entry found for object {:?}, it may have already been deallocated",
                object
            );
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            // Might mean a missing `retain` override.
            panic!(
                "Attempt to get refcount on static-lifetime object {:?}!",
                object
            );
        };
        *refcount
    }

    /// Increase the refcount of a reference-counted object. Do not call this
    /// directly unless you're implementing `release` on `NSObject`. That method
    /// may be overridden.
    pub fn increment_refcount(&mut self, object: id) {
        let Some(entry) = self.objects.get_mut(&object) else {
            panic!(
                "No entry found for object {:?}, it may have already been deallocated",
                object
            );
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            // Might mean a missing `retain` override.
            panic!(
                "Attempt to increment refcount on static-lifetime object {:?}!",
                object
            );
        };
        *refcount = refcount.checked_add(1).unwrap();
    }

    /// Decrease the refcount of a reference-counted object. Do not call this
    /// directly unless you're implementing `release` on `NSObject`. That method
    /// may be overridden.
    ///
    /// If the return value is `true`, the object needs to be deallocated. Send
    /// it the `dealloc` message.
    #[must_use]
    pub fn decrement_refcount(&mut self, object: id) -> bool {
        let Some(entry) = self.objects.get_mut(&object) else {
            panic!(
                "No entry found for object {:?}, it may have already been deallocated",
                object
            );
        };
        let Some(refcount) = entry.refcount.as_mut() else {
            // Might mean a missing `release` override.
            panic!(
                "Attempt to decrement refcount on static-lifetime object {:?}!",
                object
            );
        };
        if refcount.get() == 1 {
            entry.refcount = None;
            true
        } else {
            *refcount = NonZeroU32::new(refcount.get() - 1).unwrap();
            false
        }
    }

    /// Deallocate an object. Do not call this directly unless you're
    /// implementing `dealloc` and are sure you don't need to do a super-call.
    pub fn dealloc_object(&mut self, object: id, mem: &mut Mem) {
        let HostObjectEntry {
            host_object,
            refcount,
        } = self.objects.remove(&object).unwrap();

        if let Some(refcount) = refcount {
            // This is a serious bug if it ever happens in host code.
            // Well-behaved apps should also never do this, but Crash Bandicoot
            // Nitro Kart 3D is not a well-behaved app.
            log!(
                "Warning: {:?} was deallocated with non-zero reference count: {:?}",
                object,
                refcount
            );
        }

        std::mem::drop(host_object);

        mem.free(object.cast());
    }
}
