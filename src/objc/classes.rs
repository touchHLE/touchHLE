/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C classes and metaclasses.
//!
//! Note that metaclasses are just a special case of classes.
//!
//! Resources:
//! - [[objc explain]: Classes and metaclasses](http://www.sealiesoftware.com/blog/archive/2009/04/14/objc_explain_Classes_and_metaclasses.html), especially [the PDF diagram](http://www.sealiesoftware.com/blog/class%20diagram.pdf)

mod class_lists;
pub(super) use class_lists::CLASS_LISTS;

use super::{
    id, ivar_list_t, method_list_t, nil, objc_object, AnyHostObject, HostIMP, HostObject, ObjC,
    IMP, SEL,
};
use crate::mach_o::MachO;
use crate::mem::{guest_size_of, ConstPtr, ConstVoidPtr, GuestUSize, Mem, Ptr, SafeRead};
use std::collections::HashMap;

/// Generic pointer to an Objective-C class or metaclass.
///
/// The name is standard Objective-C.
///
/// Apple's runtime has a `objc_classes` definition similar to `objc_object`.
/// We could do the same thing here, but it doesn't seem worth it, as we can't
/// get the same unidirectional type safety.
pub type Class = id;

/// Our internal representation of a class, e.g. this is where `objc_msgSend`
/// will look up method implementations.
///
/// Note: `superclass` can be `nil`!
pub(super) struct ClassHostObject {
    pub(super) name: String,
    pub(super) is_metaclass: bool,
    pub(super) superclass: Class,
    pub(super) methods: HashMap<SEL, IMP>,
    pub(super) ivars: HashMap<String, ConstPtr<GuestUSize>>,
    /// Offset into the allocated memory for the object where the ivars of
    /// instances of this class or metaclass (respectively: normal objects or
    /// classes) should live. This is always >= the value in the superclass.
    pub(super) instance_start: GuestUSize,
    /// Size of the allocated memory for instances of this class or metaclass.
    /// This is always >= the value in the superclass.
    pub(super) instance_size: GuestUSize,
}
impl HostObject for ClassHostObject {}

/// Placeholder object for classes and metaclasses referenced by the app that
/// we don't have an implementation for.
///
/// This lets us delay errors about missing implementations until the first
/// time the app actually uses them (e.g. when a message is sent).
pub(super) struct UnimplementedClass {
    pub(super) name: String,
    pub(super) is_metaclass: bool,
}
impl HostObject for UnimplementedClass {}

/// Substitute object for classes and metaclasses from the guest app that we do
/// not want to support (see [substitute_classes]).
///
/// Messages sent to this class will behave as if messaging [nil].
pub(super) struct FakeClass {
    pub(super) name: String,
    pub(super) is_metaclass: bool,
}
impl HostObject for FakeClass {}

/// The layout of a class in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
#[allow(dead_code)]
struct class_t {
    isa: Class, // note that this matches objc_object
    superclass: Class,
    _cache: ConstVoidPtr,
    _vtable: ConstVoidPtr,
    data: ConstPtr<class_rw_t>,
}
unsafe impl SafeRead for class_t {}

/// The layout of the main class data in an app binary.
///
/// The name, field names and field layout are based on what Ghidra's output.
#[repr(C, packed)]
#[allow(dead_code)]
struct class_rw_t {
    _flags: u32,
    instance_start: GuestUSize,
    instance_size: GuestUSize,
    _reserved: u32,
    name: ConstPtr<u8>,
    base_methods: ConstPtr<method_list_t>,
    _base_protocols: ConstVoidPtr, // protocol list (TODO)
    ivars: ConstPtr<ivar_list_t>,
    _weak_ivar_layout: u32,
    _base_properties: ConstVoidPtr, // property list (TODO)
}
unsafe impl SafeRead for class_rw_t {}

/// The layout of a category in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
struct category_t {
    name: ConstPtr<u8>,
    class: Class,
    instance_methods: ConstPtr<method_list_t>,
    class_methods: ConstPtr<method_list_t>,
    _protocols: ConstVoidPtr,     // protocol list (TODO)
    _property_list: ConstVoidPtr, // property list (TODO)
}
unsafe impl SafeRead for category_t {}

/// A template for a class defined with [objc_classes].
///
/// Host implementations of libraries can use these to expose classes to the
/// application. The runtime will create the actual class ([ClassHostObject]
/// etc) from the template on-demand. See also [ClassExports].
pub struct ClassTemplate {
    pub name: &'static str,
    pub superclass: Option<&'static str>,
    pub class_methods: &'static [(&'static str, &'static dyn HostIMP)],
    pub instance_methods: &'static [(&'static str, &'static dyn HostIMP)],
}

/// Type for lists of classes exported by host implementations of frameworks.
///
/// Each module that wants to expose functions to guest code should export a
/// constant using this type. See [objc_classes] for an example.
///
/// The strings are the class names.
///
/// See also [crate::dyld::ConstantExports] and [crate::dyld::FunctionExports].
pub type ClassExports = &'static [(&'static str, ClassTemplate)];

#[doc(hidden)]
#[macro_export]
macro_rules! _objc_superclass {
    (: $name:ident) => {
        Some(stringify!($name))
    };
    () => {
        None
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _objc_method {
    (
        $env:ident,
        $this:ident,
        $_cmd:ident,
        $retty:ty,
        $block:block
        $(, $ty:ty, $arg:ident)*
        $(, ...$va_arg:ident: $va_type:ty)?
    ) => {
        // The closure must be explicitly casted because a bare closure defaults
        // to a different type than a pure fn pointer, which is the type that
        // HostIMP and CallFromGuest are implemented on.
        &((|
            #[allow(unused_variables)]
            $env: &mut $crate::Environment,
            #[allow(unused_variables)]
            $this: $crate::objc::id,
            #[allow(unused_variables)]
            $_cmd: $crate::objc::SEL,
            $($arg: $ty,)*
            $(#[allow(unused_mut)] mut $va_arg: $va_type,)?
        | -> $retty {$block}) as fn(
            &mut $crate::Environment,
            $crate::objc::id,
            $crate::objc::SEL,
            $($ty,)*
            $($va_type,)?
        ) -> $retty)
    }
}

/// Macro for creating a list of [ClassTemplate]s (i.e. [ClassExports]).
/// It imitates the Objective-C class definition syntax.
///
/// ```ignore
/// pub const CLASSES: ClassExports = objc_classes! {
/// (env, this, _cmd); // Specify names of HostIMP implicit parameters.
///                    // The second one should be `self` to match Objective-C,
///                    // but that's reserved in Rust, hence `this`.
///
/// @implementation MyClass: NSObject
///
/// + (id)foo {
///     // ...
/// }
///
/// - (id)barWithQux:(u32)qux {
///     // ...
/// }
///
/// - (id)barWithVaArgs:(u32)qux, ...dots {
///     // ...
/// }
///
/// @end
/// };
/// ```
///
/// will desugar to approximately:
///
/// ```ignore
/// pub const CLASSES: ClassExports = &[
///     ("MyClass", ClassTemplate {
///         name: "MyClass",
///         superclass: Some("NSObject"),
///         class_methods: &[
///             ("foo", &(|env: &mut Environment, this: id, _cmd: SEL| -> id {
///                 // ...
///             } as fn(&mut Environment, id, SEL) -> id)),
///         ],
///         instance_methods: &[
///             ("barWithQux:", &(|
///                 env: &mut Environment,
///                 this: id,
///                 _cmd: SEL,
///                 qux: u32
///             | -> id {
///                 // ...
///             } as &fn(&mut Environment, id, SEL, u32) -> id)),
///             ("barWithVaArgs:", &(|
///                 env: &mut Environment,
///                 this: id,
///                 _cmd: SEL,
///                 qux: u32,
///                 va_args: DotDotDot
///             | -> id {
///                 // ...
///             } as &fn(&mut Environment, id, SEL, u32, DotDotDot) -> id)),
///         ],
///     })
/// ];
/// ```
///
/// Note that the instance methods must be preceded by the class methods.
#[macro_export] // documentation comment links are annoying without this
macro_rules! objc_classes {
    {
        // Rust's macro hygiene prevents the macro's own names for these
        // parameters being visible, so we have to get names supplied by the
        // macro user.
        ($env:ident, $this:ident, $_cmd:ident);
        $(
            @implementation $class_name:ident $(: $superclass_name:ident)?

            $( + ($cm_type:ty) $cm_name:ident $(:($cm_type1:ty) $cm_arg1:ident)?
                              $($cm_namen:ident:($cm_typen:ty) $cm_argn:ident)*
                              $(, ...$cm_va_arg:ident)?
                 $cm_block:block )*

            $( - ($im_type:ty) $im_name:ident $(:($im_type1:ty) $im_arg1:ident)?
                              $($im_namen:ident:($im_typen:ty) $im_argn:ident)*
                              $(, ...$im_va_arg:ident)?
                 $im_block:block )*

            @end
        )+
    } => {
        &[
            $({
                // This constant is for `msg_super!`, which needs to know which
                // class it is has been written within (not the same as the
                // runtime type of `this`, which could be a subclass). This is
                // a constant instead of a let binding because that escapes
                // Rust's macro hygiene.
                const _OBJC_CURRENT_CLASS: &str = stringify!($class_name);

                (_OBJC_CURRENT_CLASS, $crate::objc::ClassTemplate {
                    name: _OBJC_CURRENT_CLASS,
                    superclass: $crate::_objc_superclass!($(: $superclass_name)?),
                    class_methods: &[
                        $(
                            (
                                $crate::objc::selector!(
                                    $(($cm_type1);)?
                                    $cm_name
                                    $(, $cm_namen)*
                                ),
                                $crate::_objc_method!(
                                    $env,
                                    $this,
                                    $_cmd,
                                    $cm_type,
                                    { $cm_block }
                                    $(, $cm_type1, $cm_arg1)?
                                    $(, $cm_typen, $cm_argn)*
                                    $(, ...$cm_va_arg: $crate::abi::DotDotDot)?
                                )
                            )
                        ),*
                    ],
                    instance_methods: &[
                        $(
                            (
                                $crate::objc::selector!(
                                    $(($im_type1);)?
                                    $im_name
                                    $(, $im_namen)*
                                ),
                                $crate::_objc_method!(
                                    $env,
                                    $this,
                                    $_cmd,
                                    $im_type,
                                    { $im_block }
                                    $(, $im_type1, $im_arg1)?
                                    $(, $im_typen, $im_argn)*
                                    $(, ...$im_va_arg: $crate::abi::DotDotDot)?
                                )
                            )
                        ),*
                    ],
                })
            }),+
        ]
    }
}
pub use crate::objc_classes; // #[macro_export] is weird...

impl ClassHostObject {
    fn from_template(
        template: &ClassTemplate,
        is_metaclass: bool,
        superclass: Class,
        objc: &ObjC,
    ) -> Self {
        // For our host implementations we store all data in host objects, so
        // there are no ivars and the size is always just the isa pointer.
        // This is true for both classes and normal objects.
        let size = guest_size_of::<objc_object>();
        ClassHostObject {
            name: template.name.to_string(),
            is_metaclass,
            superclass,
            methods: HashMap::from_iter(
                (if is_metaclass {
                    template.class_methods
                } else {
                    template.instance_methods
                })
                .iter()
                .map(|&(name, host_imp)| {
                    // The selector should already have been registered by
                    // [ObjC::register_host_selectors], so we can panic
                    // if it hasn't been.
                    (objc.selectors[name], IMP::Host(host_imp))
                }),
            ),
            // maybe this should be 0 for NSObject? does it matter?
            instance_start: size,
            instance_size: size,
            ivars: HashMap::default(),
        }
    }

    fn from_bin(class: Class, is_metaclass: bool, mem: &Mem, objc: &mut ObjC) -> Self {
        let class_t {
            superclass, data, ..
        } = mem.read(class.cast());
        let class_rw_t {
            instance_start,
            instance_size,
            name,
            base_methods,
            ivars,
            ..
        } = mem.read(data);

        let name = mem.cstr_at_utf8(name).unwrap().to_string();

        let mut host_object = ClassHostObject {
            name,
            is_metaclass,
            superclass,
            methods: HashMap::new(),
            instance_start,
            instance_size,
            ivars: HashMap::new(),
        };

        if !base_methods.is_null() {
            host_object.add_methods_from_bin(base_methods, mem, objc);
        }

        if !ivars.is_null() {
            host_object.add_ivars_from_bin(ivars, mem);
        }

        host_object
    }

    // See methods.rs for binary method parsing
}

/// Decide whether a certain class/metaclass pair from the guest app should use
/// fake class host objects and return the substitutions if so.
///
/// This function is called when registering classes from the guest app. It
/// detects certain problematic classes that are, for example, too complex for
/// touchHLE to currently support, but which can be easily replaced with simple
/// fakes.
fn substitute_classes(
    mem: &Mem,
    class: Class,
    metaclass: Class,
) -> Option<(Box<FakeClass>, Box<FakeClass>)> {
    let class_t { data, .. } = mem.read(class.cast());
    let class_rw_t { name, .. } = mem.read(data);
    let name = mem.cstr_at_utf8(name).unwrap();

    // Currently the only thing we try to substitute: classes that seem to be
    // from various third-party advertising SDKs. Naturally it
    // makes a lot of use of UIKit in ways we don't support yet, so it's easier
    // to skip this. This isn't "ad blocking" because ads no longer work on real
    // devices anyway :)
    if !(name.starts_with("AdMob")
        || name.starts_with("AltAds")
        || name.starts_with("Mobclix")
        || name.starts_with("Flurry"))
    {
        return None;
    }

    {
        let class_t { data, .. } = mem.read(metaclass.cast());
        let class_rw_t {
            name: metaclass_name,
            ..
        } = mem.read(data);
        let metaclass_name = mem.cstr_at_utf8(metaclass_name).unwrap();
        assert!(name == metaclass_name);
    }

    log!(
        "Note: substituting fake class for {} to improve compatibility",
        name
    );

    let class_host_object = Box::new(FakeClass {
        name: name.to_string(),
        is_metaclass: false,
    });
    let metaclass_host_object = Box::new(FakeClass {
        name: name.to_string(),
        is_metaclass: true,
    });
    Some((class_host_object, metaclass_host_object))
}

impl ObjC {
    fn get_class(&self, name: &str, is_metaclass: bool, mem: &Mem) -> Option<Class> {
        let class = self.classes.get(name).copied()?;
        Some(if is_metaclass {
            Self::read_isa(class, mem)
        } else {
            class
        })
    }

    fn find_template(name: &str) -> Option<&'static ClassTemplate> {
        crate::dyld::search_lists(CLASS_LISTS, name).map(|&(_name, ref template)| template)
    }

    /// For use by [crate::dyld]: get the class or metaclass referenced by an
    /// external relocation in the app binary. If we don't have an
    /// implementation of the class, a placeholder is used.
    pub fn link_class(&mut self, name: &str, is_metaclass: bool, mem: &mut Mem) -> Class {
        self.link_class_inner(name, is_metaclass, mem, true)
    }

    /// For use by host functions: get a particular class. If we don't have an
    /// implementation of the class, panic.
    pub fn get_known_class(&mut self, name: &str, mem: &mut Mem) -> Class {
        self.link_class_inner(name, /* is_metaclass: */ false, mem, false)
    }

    fn link_class_inner(
        &mut self,
        name: &str,
        is_metaclass: bool,
        mem: &mut Mem,
        use_placeholder: bool,
    ) -> Class {
        // The class and metaclass must be created together and tracked
        // together, so even though this function only returns one pointer, it
        // must create both. The function must not care whether the metaclass
        // is requested first, or if the class is requested first.

        if let Some(class) = self.get_class(name, is_metaclass, mem) {
            return class;
        };

        let class_host_object: Box<dyn AnyHostObject>;
        let metaclass_host_object: Box<dyn AnyHostObject>;
        if let Some(template) = Self::find_template(name) {
            // We have a template (host implementation) for this class, use it.

            if let Some(superclass_name) = template.superclass {
                // Make sure we actually have a template for the superclass
                // before we try to link it, else we might get an unimplemented
                // class back and have weird problems down the line
                assert!(Self::find_template(superclass_name).is_some());
            }

            class_host_object = Box::new(ClassHostObject::from_template(
                template,
                /* is_metaclass: */ false,
                /* superclass: */
                template
                    .superclass
                    .map(|name| {
                        self.link_class(name, /* is_metaclass: */ false, mem)
                    })
                    .unwrap_or(nil),
                self,
            ));
            metaclass_host_object = Box::new(ClassHostObject::from_template(
                template,
                /* is_metaclass: */ true,
                /* superclass: */
                template
                    .superclass
                    .map(|name| {
                        self.link_class(name, /* is_metaclass: */ true, mem)
                    })
                    .unwrap_or(nil),
                self,
            ));
        } else {
            if !use_placeholder {
                panic!("Missing implementation for class {}!", name);
            }

            // We don't have a real implementation for this class, use a
            // placeholder.

            class_host_object = Box::new(UnimplementedClass {
                name: name.to_string(),
                is_metaclass: false,
            });
            metaclass_host_object = Box::new(UnimplementedClass {
                name: name.to_string(),
                is_metaclass: true,
            });
        }

        // NSObject's metaclass is special: it is its own metaclass, and it's
        // the superclass of all other metaclasses.
        // (FIXME: this actually should apply to any class hiearchy root.)
        // This creates a chicken-and-egg problem so it has a special path.
        let metaclass = if name == "NSObject" {
            let metaclass = mem.alloc_and_write(objc_object { isa: nil });
            mem.write(metaclass, objc_object { isa: metaclass });
            self.register_static_object(metaclass, metaclass_host_object);
            metaclass
        } else {
            let isa = self.link_class("NSObject", /* is_metaclass: */ true, mem);
            self.alloc_static_object(isa, metaclass_host_object, mem)
        };

        let class = self.alloc_static_object(metaclass, class_host_object, mem);

        if name == "NSObject" {
            // NSObject's metaclass has its class as the superclass.
            self.borrow_mut::<ClassHostObject>(metaclass).superclass = class;
        }

        self.classes.insert(name.to_string(), class);

        if is_metaclass {
            metaclass
        } else {
            class
        }
    }

    /// For use by [crate::dyld]: register all the classes from the application
    /// binary.
    pub fn register_bin_classes(&mut self, bin: &MachO, mem: &mut Mem) {
        let Some(list) = bin.get_section("__objc_classlist") else {
            return;
        };

        assert!(list.size % 4 == 0);
        let base: ConstPtr<Class> = Ptr::from_bits(list.addr);
        for i in 0..(list.size / 4) {
            let class = mem.read(base + i);
            let metaclass = Self::read_isa(class, mem);

            let name = if let Some(fakes) = substitute_classes(mem, class, metaclass) {
                let (class_host_object, metaclass_host_object) = fakes;

                assert!(class_host_object.name == metaclass_host_object.name);
                let name = class_host_object.name.clone();

                self.register_static_object(class, class_host_object);
                self.register_static_object(metaclass, metaclass_host_object);
                name
            } else {
                let class_host_object = Box::new(ClassHostObject::from_bin(
                    class, /* is_metaclass: */ false, mem, self,
                ));
                let metaclass_host_object = Box::new(ClassHostObject::from_bin(
                    metaclass, /* is_metaclass: */ true, mem, self,
                ));

                assert!(class_host_object.name == metaclass_host_object.name);
                let name = class_host_object.name.clone();

                self.register_static_object(class, class_host_object);
                self.register_static_object(metaclass, metaclass_host_object);
                name
            };

            self.classes.insert(name.to_string(), class);
        }

        // Second pass to ensure no superclass has "grown into" any of its
        // subclasses.
        // TODO: Shift ivar offsets in the subclasses where it happens
        // (https://alwaysprocessing.blog/2023/03/12/objc-ivar-abi)
        for (_name, class) in self.classes.iter() {
            let class_host_object = self
                .get_host_object(*class)
                .unwrap()
                .as_any()
                .downcast_ref();
            let Some(ClassHostObject {
                superclass,
                instance_start,
                ivars,
                ..
            }) = class_host_object
            else {
                // The class might be a FakeClass or UnimplementedClass
                // In those cases we move on as they don't have ivars
                continue;
            };

            if ivars.is_empty() {
                continue;
            }

            if *superclass == nil {
                continue;
            }

            let superclass_host_object = self
                .get_host_object(*superclass)
                .unwrap()
                .as_any()
                .downcast_ref();
            let Some(ClassHostObject {
                instance_size: superclass_instance_size,
                ..
            }) = superclass_host_object
            else {
                // Superclass could also be a FakeClass or UnimplementedClass
                continue;
            };

            assert!(instance_start >= superclass_instance_size);
        }
    }

    /// For use by [crate::dyld]: register all the categories from the
    /// application binary.
    pub fn register_bin_categories(&mut self, bin: &MachO, mem: &mut Mem) {
        let Some(list) = bin.get_section("__objc_catlist") else {
            return;
        };

        assert!(list.size % 4 == 0);
        let base: ConstPtr<ConstPtr<category_t>> = Ptr::from_bits(list.addr);
        for i in 0..(list.size / 4) {
            let cat_ptr = mem.read(base + i);
            let data = mem.read(cat_ptr);

            let name = mem.cstr_at_utf8(data.name).unwrap();
            let class = data.class;
            let metaclass = Self::read_isa(class, mem);

            for (class, methods) in [
                (class, data.instance_methods),
                (metaclass, data.class_methods),
            ] {
                if methods.is_null() {
                    continue;
                }

                let any = self.get_host_object(class).unwrap().as_any();
                if any.is::<FakeClass>() || any.is::<UnimplementedClass>() {
                    continue;
                }

                // Horrible workaround to avoid double-borrowing self:
                // temporarily replace the class object.
                let mut host_obj = std::mem::replace(
                    self.borrow_mut::<ClassHostObject>(class),
                    ClassHostObject {
                        name: Default::default(),
                        is_metaclass: Default::default(),
                        superclass: nil,
                        methods: Default::default(),
                        instance_start: Default::default(),
                        instance_size: Default::default(),
                        ivars: Default::default(),
                    },
                );
                log_dbg!(
                    "Adding {} methods from guest app category \"{}\" {:?} to {} \"{}\" {:?}",
                    if host_obj.is_metaclass {
                        "class"
                    } else {
                        "instance"
                    },
                    name,
                    cat_ptr,
                    if host_obj.is_metaclass {
                        "metaclass"
                    } else {
                        "class"
                    },
                    host_obj.name,
                    class,
                );
                host_obj.add_methods_from_bin(methods, mem, self);
                *self.borrow_mut::<ClassHostObject>(class) = host_obj;
            }
        }
    }

    pub fn class_is_subclass_of(&self, class: Class, superclass: Class) -> bool {
        if class == superclass {
            return true;
        }

        let mut class = class;
        loop {
            let &ClassHostObject {
                superclass: next, ..
            } = self.borrow(class);
            if next == nil {
                return false;
            } else if next == superclass {
                return true;
            } else {
                class = next;
            }
        }
    }

    pub fn get_class_name(&self, class: Class) -> &str {
        let host_object = self.get_host_object(class).unwrap();
        if let Some(ClassHostObject { name, .. }) = host_object.as_any().downcast_ref() {
            name
        } else if let Some(UnimplementedClass { name, .. }) = host_object.as_any().downcast_ref() {
            name
        } else if let Some(FakeClass { name, .. }) = host_object.as_any().downcast_ref() {
            name
        } else {
            panic!();
        }
    }
}
