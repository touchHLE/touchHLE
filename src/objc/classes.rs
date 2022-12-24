//! Handling of Objective-C classes and metaclasses.
//!
//! Note that metaclasses are just a special case of classes.
//!
//! Resources:
//! - [[objc explain]: Classes and metaclasses](http://www.sealiesoftware.com/blog/archive/2009/04/14/objc_explain_Classes_and_metaclasses.html), especially [the PDF diagram](http://www.sealiesoftware.com/blog/class%20diagram.pdf)

mod class_lists;
pub(super) use class_lists::CLASS_LISTS;

use super::{id, nil, AnyHostObject, HostIMP, HostObject, ObjC, IMP, SEL};
use crate::mem::Mem;
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
/// Once we can load classes from the app itself (TODO), we will need to create
/// these objects for each.
///
/// Note: `superclass` can be `nil`!
pub(super) struct ClassHostObject {
    pub(super) name: String,
    pub(super) is_metaclass: bool,
    pub(super) superclass: Class,
    pub(super) methods: HashMap<SEL, IMP>,
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
/// See also [crate::dyld::FunctionExports].
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
        $block:tt
        $(, $ty:ty, $arg:ident)*
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
        | -> $retty $block) as fn(
            &mut $crate::Environment,
            $crate::objc::id,
            $crate::objc::SEL,
            $($ty,)*
        ) -> $retty)
    }
}

/// Macro for creating a list of [ClassTemplate]s (i.e. [ClassExports]).
/// It imitates the Objective-C class definition syntax.
///
/// ```rust
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
/// @end
/// };
/// ```
///
/// will desugar to approximately:
///
/// ```rust
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
///             ("barWithQux:", &(|env: &mut Environment, this: id, _cmd: SEL, qux: u32| -> id {
///                 // ...
///             } as &fn(&mut Environment, id, SEL, u32) -> id)),
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
                 $cm_block:block )*

            $( - ($im_type:ty) $im_name:ident $(:($im_type1:ty) $im_arg1:ident)?
                              $($im_namen:ident:($im_typen:ty) $im_argn:ident)*
                 $im_block:block )*

            @end
        )+
    } => {
        &[
            $(
                (stringify!($class_name), $crate::objc::ClassTemplate {
                    name: stringify!($class_name),
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
                                )
                            )
                        ),*
                    ],
                })
            ),+
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
        }
    }
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
        crate::dyld::search_lists(CLASS_LISTS, name)
    }

    /// For use by [crate::dyld]: get the class referenced by an external
    /// relocation in the application.
    pub fn link_class(&mut self, name: &str, is_metaclass: bool, mem: &mut Mem) -> Class {
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

        // the metaclass's isa can't be nil, so it should point back to the
        // metaclass, but we can't make the object self-referential in a single
        // step, so: write nil and then overwrite it.
        let metaclass = self.alloc_object(nil, metaclass_host_object, mem);
        Self::write_isa(metaclass, metaclass, mem);

        let class = self.alloc_object(metaclass, class_host_object, mem);

        self.classes.insert(name.to_string(), class);

        if is_metaclass {
            metaclass
        } else {
            class
        }
    }
}
