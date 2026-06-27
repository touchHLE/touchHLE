/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Auto Layout / UIFontDescriptor / NSLocalizableString placeholders.
//!
//! HyperHLE renders the screen using a frame-based UI (everything is
//! positioned through `frame`, `bounds` and `center` on `UIView`). It does
//! not implement Apple's Cascade Auto Layout engine. However, modern
//! storyboards and xibs frequently reference layout-related classes from
//! within their NIB archive — most notably:
//!
//! * `NSLayoutConstraint` (a constraint between two anchors),
//! * `_UILayoutGuide` (a non-drawing layout area, e.g. the top/bottom
//!   layout guide of a view controller),
//! * `_UILayoutSupportConstraint` (a private constraint helper Apple uses
//!   to anchor the layout guides),
//! * `UIFontDescriptor` (font information stored in nibs).
//!
//! Without `initWithCoder:` implementations for those classes the NIB
//! decoder raises `does not respond to selector initWithCoder:` and the
//! whole storyboard scene fails to load. The implementations here record
//! every decoded attribute on a host-side dictionary so that introspection
//! and KVC still work, but they intentionally do not run a constraint
//! solver — the view controllers receive plain `UIView`s that use the
//! frames declared elsewhere in the nib instead.
//!
//! `NSLocalizableString` is exported by the legacy Foundation private API
//! as a thin `NSString` subclass; we keep behaviour parity by inheriting
//! straight from `NSString` and forwarding `initWithCoder:` to the
//! existing NSString unarchival path.

use crate::frameworks::core_graphics::CGFloat;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

#[derive(Default)]
struct NSLayoutConstraintHostObject {
    first_item: id,
    first_attribute: i32,
    relation: i32,
    second_item: id,
    second_attribute: i32,
    multiplier: CGFloat,
    constant: CGFloat,
    priority: f32,
    identifier: id,
    active: bool,
}
impl HostObject for NSLayoutConstraintHostObject {}

#[derive(Default)]
struct UILayoutGuideHostObject {
    /// Owning view (non-retained), set when the guide is added to a view
    /// via `-addLayoutGuide:`. Always nil under the no-op constraint
    /// engine.
    owning_view: id,
    identifier: id,
    /// Most code that asks for `frame` immediately afterwards is happy
    /// with `CGRectZero`; we still store any frame the nib decoded so
    /// `setFrame:` round-trips.
    frame_origin_x: CGFloat,
    frame_origin_y: CGFloat,
    frame_size_w: CGFloat,
    frame_size_h: CGFloat,
}
impl HostObject for UILayoutGuideHostObject {}

#[derive(Default)]
struct UIFontDescriptorHostObject {
    attributes: id,
    point_size: CGFloat,
}
impl HostObject for UIFontDescriptorHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - NSLayoutConstraint
// =========================================================================

@implementation NSLayoutConstraint: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<NSLayoutConstraintHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

// Bulk activation helpers used by NSLayoutConstraint-heavy code.
+ (())activateConstraints:(id)constraints {
    if constraints == nil { return; }
    let count: crate::frameworks::foundation::NSUInteger = msg![env; constraints count];
    for i in 0..count {
        let c: id = msg![env; constraints objectAtIndex:i];
        let _: () = msg![env; c setActive:true];
    }
}
+ (())deactivateConstraints:(id)constraints {
    if constraints == nil { return; }
    let count: crate::frameworks::foundation::NSUInteger = msg![env; constraints count];
    for i in 0..count {
        let c: id = msg![env; constraints objectAtIndex:i];
        let _: () = msg![env; c setActive:false];
    }
}

- (id)initWithCoder:(id)coder {
    let first_item_key = get_static_str(env, "NSFirstItem");
    let first_item: id = msg![env; coder decodeObjectForKey:first_item_key];
    if first_item != nil { retain(env, first_item); }

    let second_item_key = get_static_str(env, "NSSecondItem");
    let second_item: id = msg![env; coder decodeObjectForKey:second_item_key];
    if second_item != nil { retain(env, second_item); }

    let first_attr_key = get_static_str(env, "NSFirstAttribute");
    let first_attr: i32 = msg![env; coder decodeIntForKey:first_attr_key];

    let second_attr_key = get_static_str(env, "NSSecondAttribute");
    let second_attr: i32 = msg![env; coder decodeIntForKey:second_attr_key];

    let relation_key = get_static_str(env, "NSRelation");
    let relation: i32 = msg![env; coder decodeIntForKey:relation_key];

    let multiplier_key = get_static_str(env, "NSMultiplier");
    let multiplier: f32 = msg![env; coder decodeFloatForKey:multiplier_key];

    let constant_key = get_static_str(env, "NSConstant");
    let constant: f32 = msg![env; coder decodeFloatForKey:constant_key];

    let priority_key = get_static_str(env, "NSPriority");
    let mut priority: f32 = msg![env; coder decodeFloatForKey:priority_key];
    if priority == 0.0 { priority = 1000.0; }

    let identifier_key = get_static_str(env, "NSIdentifier");
    let identifier: id = msg![env; coder decodeObjectForKey:identifier_key];
    if identifier != nil { retain(env, identifier); }

    {
        let host = env.objc.borrow_mut::<NSLayoutConstraintHostObject>(this);
        host.first_item = first_item;
        host.first_attribute = first_attr;
        host.second_item = second_item;
        host.second_attribute = second_attr;
        host.relation = relation;
        host.multiplier = multiplier;
        host.constant = constant;
        host.priority = priority;
        host.identifier = identifier;
        host.active = true;
    }
    this
}

- (id)firstItem {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).first_item
}
- (id)secondItem {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).second_item
}
- (i32)firstAttribute {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).first_attribute
}
- (i32)secondAttribute {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).second_attribute
}
- (i32)relation {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).relation
}
- (CGFloat)multiplier {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).multiplier
}
- (CGFloat)constant {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).constant
}
- (())setConstant:(CGFloat)c {
    env.objc.borrow_mut::<NSLayoutConstraintHostObject>(this).constant = c;
}
- (f32)priority {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).priority
}
- (())setPriority:(f32)p {
    env.objc.borrow_mut::<NSLayoutConstraintHostObject>(this).priority = p;
}
- (id)identifier {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).identifier
}
- (())setIdentifier:(id)identifier {
    if identifier != nil { retain(env, identifier); }
    let host = env.objc.borrow_mut::<NSLayoutConstraintHostObject>(this);
    let old = host.identifier;
    host.identifier = identifier;
    if old != nil { release(env, old); }
}
- (bool)isActive {
    env.objc.borrow::<NSLayoutConstraintHostObject>(this).active
}
- (())setActive:(bool)active {
    env.objc.borrow_mut::<NSLayoutConstraintHostObject>(this).active = active;
}

- (())dealloc {
    let &NSLayoutConstraintHostObject { first_item, second_item, identifier, .. } =
        env.objc.borrow(this);
    if first_item != nil { release(env, first_item); }
    if second_item != nil { release(env, second_item); }
    if identifier != nil { release(env, identifier); }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - _UILayoutSupportConstraint
// =========================================================================
//
// `_UILayoutSupportConstraint` is a private Apple subclass of
// `NSLayoutConstraint`. Behaviour is identical for our purposes.

@implementation _UILayoutSupportConstraint: NSLayoutConstraint
@end

// =========================================================================
// MARK: - _UILayoutGuide / UILayoutGuide
// =========================================================================
//
// `_UILayoutGuide` is the legacy private name (pre-iOS 9). `UILayoutGuide`
// is the public class introduced in iOS 9. Both behave as non-drawing
// guides positioned via Auto Layout. HyperHLE does not run a constraint
// solver, so the guides remain at zero frame; storyboards that reference
// `topLayoutGuide` / `bottomLayoutGuide` still decode correctly though.

@implementation _UILayoutGuide: UIView
@end

@implementation UILayoutGuide: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UILayoutGuideHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (id)init { this }

- (id)initWithCoder:(id)coder {
    let identifier_key = get_static_str(env, "UILayoutGuideIdentifier");
    let identifier: id = msg![env; coder decodeObjectForKey:identifier_key];
    if identifier != nil { retain(env, identifier); }
    env.objc.borrow_mut::<UILayoutGuideHostObject>(this).identifier = identifier;
    this
}

- (id)identifier {
    env.objc.borrow::<UILayoutGuideHostObject>(this).identifier
}
- (())setIdentifier:(id)identifier {
    if identifier != nil { retain(env, identifier); }
    let host = env.objc.borrow_mut::<UILayoutGuideHostObject>(this);
    let old = host.identifier;
    host.identifier = identifier;
    if old != nil { release(env, old); }
}

- (id)owningView {
    env.objc.borrow::<UILayoutGuideHostObject>(this).owning_view
}
- (())setOwningView:(id)view {
    env.objc.borrow_mut::<UILayoutGuideHostObject>(this).owning_view = view;
}

- (crate::frameworks::core_graphics::CGRect)layoutFrame {
    let host = env.objc.borrow::<UILayoutGuideHostObject>(this);
    crate::frameworks::core_graphics::CGRect {
        origin: crate::frameworks::core_graphics::CGPoint {
            x: host.frame_origin_x,
            y: host.frame_origin_y,
        },
        size: crate::frameworks::core_graphics::CGSize {
            width: host.frame_size_w,
            height: host.frame_size_h,
        },
    }
}

- (())dealloc {
    let &UILayoutGuideHostObject { identifier, .. } = env.objc.borrow(this);
    if identifier != nil { release(env, identifier); }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - UIFontDescriptor
// =========================================================================

@implementation UIFontDescriptor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::<UIFontDescriptorHostObject>::default();
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)fontDescriptorWithFontAttributes:(id)attributes {
    let descriptor: id = msg![env; this alloc];
    if attributes != nil { retain(env, attributes); }
    env.objc.borrow_mut::<UIFontDescriptorHostObject>(descriptor).attributes = attributes;
    autorelease(env, descriptor)
}

+ (id)fontDescriptorWithName:(id)name size:(CGFloat)size {
    let attrs: id = msg_class![env; NSMutableDictionary dictionary];
    let name_key = get_static_str(env, "NSFontNameAttribute");
    let _: () = msg![env; attrs setObject:name forKey:name_key];
    let descriptor: id = msg![env; this fontDescriptorWithFontAttributes:attrs];
    env.objc.borrow_mut::<UIFontDescriptorHostObject>(descriptor).point_size = size;
    descriptor
}

- (id)initWithCoder:(id)coder {
    let attrs_key = get_static_str(env, "UIFontDescriptorAttributes");
    let attrs: id = msg![env; coder decodeObjectForKey:attrs_key];
    if attrs != nil { retain(env, attrs); }
    env.objc.borrow_mut::<UIFontDescriptorHostObject>(this).attributes = attrs;

    let size_key = get_static_str(env, "UIFontDescriptorPointSize");
    let size: f32 = msg![env; coder decodeFloatForKey:size_key];
    env.objc.borrow_mut::<UIFontDescriptorHostObject>(this).point_size = size;
    this
}

- (id)fontAttributes {
    env.objc.borrow::<UIFontDescriptorHostObject>(this).attributes
}
- (CGFloat)pointSize {
    env.objc.borrow::<UIFontDescriptorHostObject>(this).point_size
}
- (id)objectForKey:(id)key {
    let attrs = env.objc.borrow::<UIFontDescriptorHostObject>(this).attributes;
    if attrs == nil { return nil; }
    msg![env; attrs objectForKey:key]
}
- (id)fontDescriptorWithSize:(CGFloat)size {
    let class = msg![env; this class];
    let attrs = env.objc.borrow::<UIFontDescriptorHostObject>(this).attributes;
    let descriptor: id = msg![env; class fontDescriptorWithFontAttributes:attrs];
    env.objc.borrow_mut::<UIFontDescriptorHostObject>(descriptor).point_size = size;
    descriptor
}

- (())dealloc {
    let &UIFontDescriptorHostObject { attributes, .. } = env.objc.borrow(this);
    if attributes != nil { release(env, attributes); }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// =========================================================================
// MARK: - NSLocalizableString
// =========================================================================
//
// Some old Apple SDKs ship a private NSLocalizableString class that
// behaves as a thin NSString subclass which decodes from `NS.bytes`.
// Treat it as an alias of NSString so storyboard-encoded localized
// strings continue to work.

@implementation NSLocalizableString: NSString
@end

};
