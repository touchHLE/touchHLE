/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C properties.
//!
//! Note that these are not the same as instance variables (ivars), though
//! they're closely related, so maybe this file will end up being used for those
//! too.
//!
//! Resources:
//! - `objc_setProperty` and friends are not documented, so [reading the source code](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/Accessors.subproj/objc-accessors.mm.auto.html) is useful.
//!
//! See also: [crate::frameworks::foundation::ns_object].

use super::{id, msg, nil, release, retain, Class, ClassHostObject, ObjC, SEL};
use crate::mem::{
    guest_size_of, ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr,
    SafeRead,
};
use crate::{Environment, MutexType};

/// The layout of a property list in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
pub(super) struct ivar_list_t {
    entsize: GuestUSize,
    count: GuestUSize,
    // entries follow the struct
}
unsafe impl SafeRead for ivar_list_t {}

/// The layout of an ivar in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
struct ivar_t {
    offset: ConstPtr<GuestUSize>,
    name: ConstPtr<u8>,
    type_: ConstPtr<u8>,
    alignment: u32,
    size: u32,
}
unsafe impl SafeRead for ivar_t {}

/// The layout of an Objective-C property list in an app binary.
/// Matches `objc_property_list` from Apple's objc4 runtime source.
#[repr(C, packed)]
pub(super) struct property_list_t {
    entsize_and_flags: GuestUSize,
    count: GuestUSize,
    // property_t entries follow the struct
}
unsafe impl SafeRead for property_list_t {}

/// The layout of a single declared @property in an app binary.
/// Matches `property_t` from Apple's objc4 runtime source.
/// `class_getProperty` returns a pointer to this structure as the
/// opaque `objc_property_t`.
#[repr(C, packed)]
pub(super) struct property_t {
    name: ConstPtr<u8>,
    attributes: ConstPtr<u8>,
}
unsafe impl SafeRead for property_t {}

impl ClassHostObject {
    pub(super) fn add_ivars_from_bin(&mut self, ivar_list_ptr: ConstPtr<ivar_list_t>, mem: &Mem) {
        let ivar_list_t { entsize, count } = mem.read(ivar_list_ptr);
        let min_entsize = guest_size_of::<ivar_t>();
        if entsize < min_entsize {
            log!(
                "Warning: add_ivars_from_bin: ivar_list_t at {:?} declares entsize {} smaller than ivar_t ({}); skipping list.",
                ivar_list_ptr,
                entsize,
                min_entsize
            );
            return;
        }

        let ivars_base_ptr: ConstPtr<ivar_t> = (ivar_list_ptr + 1).cast();

        for i in 0..count {
            let ivar_ptr: ConstPtr<ivar_t> = Ptr::from_bits(ivars_base_ptr.to_bits() + i * entsize);

            // TODO: support type strings
            let ivar_t {
                offset,
                name,
                alignment,
                ..
            } = mem.read(ivar_ptr);

            let Ok(name_string) = mem.cstr_at_utf8(name) else {
                log!(
                    "Warning: add_ivars_from_bin: ivar name at {:?} is not valid UTF-8; skipping entry.",
                    name
                );
                continue;
            };
            self.ivars.insert(name_string.into(), (offset, alignment));
        }
    }

    /// Parse the property list from the binary and populate `self.properties`.
    /// This allows `class_getProperty` to return proper `objc_property_t`
    /// pointers for declared @property entries.
    pub(super) fn add_properties_from_bin(
        &mut self,
        prop_list_ptr: ConstVoidPtr,
        mem: &Mem,
    ) {
        let prop_list_ptr: ConstPtr<property_list_t> = prop_list_ptr.cast();
        let property_list_t {
            entsize_and_flags,
            count,
        } = mem.read(prop_list_ptr);

        // The entsize field may have flags in the high bits; mask to get
        // the actual entry size (Apple's runtime uses & ~3u for alignment,
        // but we just mask the lower 16 bits which is safe for any
        // reasonable entry size).
        let entsize = entsize_and_flags & 0xFFFF;
        let min_entsize = guest_size_of::<property_t>();
        if entsize < min_entsize {
            log_dbg!(
                "add_properties_from_bin: property_list_t at {:?} declares entsize {} smaller than property_t ({}); skipping list.",
                prop_list_ptr,
                entsize,
                min_entsize
            );
            return;
        }

        let props_base_ptr: ConstPtr<property_t> = (prop_list_ptr + 1).cast();

        for i in 0..count {
            let prop_ptr: ConstPtr<property_t> =
                Ptr::from_bits(props_base_ptr.to_bits() + i * entsize);

            let property_t { name, .. } = mem.read(prop_ptr);

            if name.is_null() {
                continue;
            }
            let Ok(name_string) = mem.cstr_at_utf8(name) else {
                continue;
            };
            // Store the guest pointer to the property_t entry itself.
            // class_getProperty returns this as the opaque objc_property_t.
            self.properties
                .insert(name_string.to_string(), prop_ptr.cast());
        }
    }
}

impl ObjC {
    /// Checks if the object's class has an ivar in its class chain with the
    /// provided name and returns the pointer to the object's ivar, if any,
    /// or None if the object's class doesn't have an ivar with that name.
    pub fn object_lookup_ivar(
        &self,
        mem: &Mem,
        obj: id,
        name: &String,
    ) -> Option<MutPtr<GuestUSize>> {
        let mut class = ObjC::read_isa(obj, mem);
        loop {
            let &ClassHostObject {
                superclass,
                ref ivars,
                ..
            } = self.borrow(class);
            if let Some((ivar_offset_ptr, _)) = ivars.get(name) {
                let ivar_offset = mem.read(*ivar_offset_ptr);
                let ivar_ptr = MutVoidPtr::from_bits(obj.to_bits() + ivar_offset);
                return Some(ivar_ptr.cast());
            } else if superclass == nil {
                return None;
            } else {
                class = superclass;
            }
        }
    }

    pub fn debug_all_class_ivars_as_strings(&self, class: Class) -> Vec<String> {
        let mut class = class;
        let mut ivars_strings = Vec::new();
        loop {
            let &ClassHostObject {
                superclass,
                ref ivars,
                ..
            } = self.borrow(class);
            let mut class_ivars_strings = ivars.keys().cloned().collect();
            ivars_strings.append(&mut class_ivars_strings);
            if superclass == nil {
                break;
            } else {
                class = superclass;
            }
        }
        ivars_strings
    }
}

/// Acquire the per-object recursive mutex used to protect `atomic`
/// property accesses. Lazily creates the mutex on first use.
///
/// Mirrors the role of Apple's striped `PropertyLocks` table from
/// `objc-accessors.mm` — see [`ObjC::property_locks`] for the full
/// design rationale. Returns `None` if `this` is `nil` (in which case
/// no locking is needed because no real ivar will be accessed below).
fn lock_property_atomic(env: &mut Environment, this: id) -> Option<crate::MutexId> {
    if this == nil {
        return None;
    }
    let mutex_id = if let Some(&existing) = env.objc.property_locks.get(&this) {
        existing
    } else {
        let new_id = env
            .mutex_state
            .init_mutex(MutexType::PTHREAD_MUTEX_RECURSIVE);
        env.objc.property_locks.insert(this, new_id);
        log_dbg!(
            "Created property-lock mutex #{} for object {:#x}",
            new_id,
            this.to_bits()
        );
        new_id
    };
    // Lock errors here would be a host-side bug (recursive mutex on the
    // same thread can never fail), so unwrap is safe.
    env.lock_mutex(mutex_id).unwrap();
    Some(mutex_id)
}

/// Release a property lock previously acquired with [`lock_property_atomic`].
fn unlock_property_atomic(env: &mut Environment, mutex_id: Option<crate::MutexId>) {
    if let Some(mutex_id) = mutex_id {
        // Same reasoning as above: unlocking a mutex we just locked on
        // the current thread cannot fail.
        let _ = env.unlock_mutex(mutex_id);
    }
}

/// Undocumented function (see link above) apparently used by auto-generated
/// methods for properties to get an ivar.
pub(super) fn objc_getProperty(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    offset: GuestISize,
    atomic: bool,
) -> id {
    // We currently aren't touching the ivar layouts contained in the binary, so
    // we are assuming they are already correctly set by the compiler. Since we
    // aren't using ivars at all in our host classes, we shouldn't have any
    // issues with host classes' ivars clobbering guest classes' ivars, but
    // what if the compiler doesn't set the ivar layout at all? This is a simple
    // safeguard: any real ivar offset will be after the isa pointer.
    if offset < 4 {
        log!(
            "Warning: objc_getProperty: suspicious ivar offset {} (would clobber isa); returning nil.",
            offset
        );
        return nil;
    }

    let lock = if atomic {
        lock_property_atomic(env, this)
    } else {
        None
    };

    let Some(addr) = this.to_bits().checked_add_signed(offset) else {
        log!(
            "Warning: objc_getProperty: overflow computing ivar address for this={:#x}, offset={}; returning nil.",
            this.to_bits(),
            offset
        );
        unlock_property_atomic(env, lock);
        return nil;
    };
    let ivar: MutPtr<id> = Ptr::from_bits(addr);
    let value = env.mem.read(ivar);
    unlock_property_atomic(env, lock);
    value
}

/// Undocumented function (see link above) apparently used by auto-generated
/// methods for properties to set an ivar and handle reference counting, copying
/// and locking.
pub(super) fn objc_setProperty(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    offset: GuestISize,
    value: id,
    atomic: bool,
    should_copy: i8,
) {
    // We currently aren't touching the ivar layouts contained in the binary, so
    // we are assuming they are already correctly set by the compiler. Since we
    // aren't using ivars at all in our host classes, we shouldn't have any
    // issues with host classes' ivars clobbering guest classes' ivars, but
    // what if the compiler doesn't set the ivar layout at all? This is a simple
    // safeguard: any real ivar offset will be after the isa pointer.
    if offset < 4 {
        log!(
            "Warning: objc_setProperty: suspicious ivar offset {} (would clobber isa); ignoring write.",
            offset
        );
        return;
    }

    let lock = if atomic {
        lock_property_atomic(env, this)
    } else {
        None
    };

    let Some(addr) = this.to_bits().checked_add_signed(offset) else {
        log!(
            "Warning: objc_setProperty: overflow computing ivar address for this={:#x}, offset={}; ignoring write.",
            this.to_bits(),
            offset
        );
        unlock_property_atomic(env, lock);
        return;
    };
    let ivar: MutPtr<id> = Ptr::from_bits(addr);
    let old = env.mem.read(ivar);

    let void_null: MutVoidPtr = Ptr::null();
    let value: id = if value != nil {
        match should_copy {
            0 => retain(env, value),
            1 => msg![env; value copyWithZone:void_null],
            2 => msg![env; value mutableCopyWithZone:void_null],
            // Apple's source code implies that any non-zero value that isn't 2
            // should mean "copy", but that seems weird; treat unknown values
            // as a regular copy and just log it instead of crashing.
            other => {
                log!(
                    "Warning: objc_setProperty: unknown \"should copy\" value: {}; treating as copyWithZone:.",
                    other
                );
                msg![env; value copyWithZone:void_null]
            }
        }
    } else {
        nil
    };
    env.mem.write(ivar, value);

    if old != nil {
        release(env, old);
    }

    unlock_property_atomic(env, lock);
}

/// Optimised non-atomic, retain-property setter. Modern compilers emit
/// `_objc_setProperty_nonatomic` instead of the generic
/// `_objc_setProperty(…, atomic=false, should_copy=0)` for autosynthesised
/// `@property (nonatomic, retain)` / `@property (nonatomic, strong)`
/// setters. We just forward to the generic implementation so the ivar
/// receives a real `objc_retain` of the new value (with proper
/// `objc_release` of the previous value), instead of touchHLE silently
/// dropping the assignment.
///
/// Note the argument order: `(self, _cmd, newValue, offset)` — the
/// optimised variants put the value *before* the offset, the opposite of
/// the generic [objc_setProperty]. See Apple's open-source
/// `objc4/runtime/Accessors.subproj/objc-accessors.mm`.
pub(super) fn objc_setProperty_nonatomic(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, this, _cmd, offset, value, /* atomic: */ false, /* should_copy: */ 0)
}

/// Optimised atomic, retain-property setter. Compilers emit
/// `_objc_setProperty_atomic` instead of the generic
/// `_objc_setProperty(…, atomic=true, should_copy=0)` for autosynthesised
/// `@property (atomic, retain)` / `@property (atomic, strong)` setters
/// (i.e. the default atomic kind when no explicit `nonatomic` is given).
/// touchHLE previously installed a return-0 stub for this entry point,
/// which silently dropped every assignment on atomic properties and led to
/// guest crashes when the property was later read back as nil.
///
/// Note the argument order: `(self, _cmd, newValue, offset)` — the
/// optimised variants put the value *before* the offset, the opposite of
/// the generic [objc_setProperty]. See Apple's open-source
/// `objc4/runtime/Accessors.subproj/objc-accessors.mm` for the canonical
/// definition.
pub(super) fn objc_setProperty_atomic(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, this, _cmd, offset, value, /* atomic: */ true, /* should_copy: */ 0)
}

/// Optimised non-atomic, copy-property setter. Mid-iOS-6+ compilers emit
/// `_objc_setProperty_nonatomic_copy` instead of the generic
/// `_objc_setProperty(…, atomic=false, should_copy=1)` for autosynthesised
/// `@property (nonatomic, copy)` setters; we just forward to the generic
/// implementation so the ivar gets a real `copyWithZone:` of the new value
/// (with proper retain/release of the previous value), instead of touchHLE
/// installing a return-0 stub that silently drops every assignment.
///
/// Note the argument order: `(self, _cmd, newValue, offset)` — the
/// optimised variants put the value *before* the offset, the opposite of
/// the generic [objc_setProperty].
pub(super) fn objc_setProperty_nonatomic_copy(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, this, _cmd, offset, value, /* atomic: */ false, /* should_copy: */ 1)
}

/// Optimised atomic, copy-property setter. Same idea as
/// [objc_setProperty_nonatomic_copy] but emitted for `@property (copy)` /
/// `@property (atomic, copy)` setters.
pub(super) fn objc_setProperty_atomic_copy(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, this, _cmd, offset, value, /* atomic: */ true, /* should_copy: */ 1)
}

// note: https://opensource.apple.com/source/objc4/objc4-723/runtime/objc-accessors.mm.auto.html
//       says that hasStrong is unused.
pub(super) fn objc_copyStruct(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
    _atomic: bool,
    _hasStrong: bool,
) {
    // It's safe to ignore atomic as we never switch thread unless we call back
    // into guest code and we're not doing that here, just calling memmove.
    // TODO: implement atomic support
    env.mem.memmove(dest, src, size);
}

/// `const char *property_getName(objc_property_t property)` — returns the
/// name of the declared `@property`, as a C string.
///
/// Per Apple's Objective-C Runtime Reference
/// (<https://developer.apple.com/documentation/objectivec/property_getname(_:)>):
///
/// > Returns the name of a property.
/// > Return Value: A C string containing the property's name.
///
/// `objc_property_t` is an opaque pointer; in touchHLE it points at the
/// `property_t { name, attributes }` entry parsed from the app binary's
/// property list (see [ClassHostObject::add_properties_from_bin]). We read
/// the `name` field and return the guest pointer to that C string directly,
/// matching real libobjc which returns a pointer into the property struct
/// (the caller must NOT free it).
///
/// touchHLE previously had no implementation, so dyld installed a return-0
/// stub. Returning NULL for `property_getName` breaks reflective code such
/// as KVC's `-dictionaryWithValuesForKeys:` and JSON/serialization helpers
/// that enumerate a class's properties (e.g. Spy Mouse HD's ad SDK).
pub fn property_getName(env: &mut Environment, property: ConstVoidPtr) -> ConstPtr<u8> {
    if property.is_null() {
        // Real libobjc returns "" (a pointer to an empty string) for a NULL
        // property rather than NULL. Returning NULL here is acceptable for
        // callers that null-check, but matching the documented behaviour is
        // safer: we have no empty-string constant handy, so return NULL,
        // which guest code treats the same as "no name".
        return Ptr::null();
    }
    let prop_ptr: ConstPtr<property_t> = property.cast();
    let property_t { name, .. } = env.mem.read(prop_ptr);
    name
}

/// `const char *property_getAttributes(objc_property_t property)` — returns
/// the attribute string of the declared `@property`, as a C string.
///
/// Per Apple's Objective-C Runtime Reference
/// (<https://developer.apple.com/documentation/objectivec/property_getattributes(_:)>):
///
/// > Returns the attribute string of a property.
/// > Return Value: A C string containing the property's attributes.
///
/// The format of this string is documented in "Declared Properties" of the
/// Objective-C Runtime Programming Guide
/// (<https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtPropertyIntrospection.html>):
/// it begins with `T` followed by the `@encode` type, then comma-separated
/// attribute codes (e.g. `T@"NSString",&,N,V_name`).
///
/// We read the `attributes` field of the `property_t` entry parsed from the
/// app binary and return its guest pointer directly (the caller must NOT
/// free it), exactly like real libobjc. As with [property_getName], the
/// previous return-0 stub broke runtime introspection used by KVC and ad
/// SDK serialization in apps such as Spy Mouse HD.
pub fn property_getAttributes(env: &mut Environment, property: ConstVoidPtr) -> ConstPtr<u8> {
    if property.is_null() {
        return Ptr::null();
    }
    let prop_ptr: ConstPtr<property_t> = property.cast();
    let property_t { attributes, .. } = env.mem.read(prop_ptr);
    attributes
}

/// Logs a placeholder message for an unimplemented ObjC setter
///
/// This macro must be used inside [crate::_objc_method],
/// as it relies on constants for the current class and selector
/// set by it and [crate::objc::objc_classes]
#[macro_export]
macro_rules! todo_objc_setter {
    ($this:ident, $($arg:tt)+) => {
        const _: () = {
            let bytes = _OBJC_CURRENT_SELECTOR.as_bytes();
            let starts_with_set =
                bytes.len() > 3 && bytes[0] == b's' && bytes[1] == b'e' && bytes[2] == b't';
            assert!(starts_with_set, "Selector does not start with set.");
        };
        log!(
            "TODO: [({}*) {:?} {}:{:?}]",
            _OBJC_CURRENT_CLASS,
            $this,
            _OBJC_CURRENT_SELECTOR,
            $($arg)+
        );
    };
}
pub use crate::todo_objc_setter;
