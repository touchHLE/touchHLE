/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISegmentedControl`.

use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::{send_actions, UIControlEventValueChanged};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, HostObject, NSZonePtr,
};

type UISegmentedControlStyle = NSInteger;
#[allow(dead_code)]
const UISegmentedControlStylePlain: UISegmentedControlStyle = 0;
#[allow(dead_code)]
const UISegmentedControlStyleBordered: UISegmentedControlStyle = 1;
#[allow(dead_code)]
const UISegmentedControlStyleBar: UISegmentedControlStyle = 2;
#[allow(dead_code)]
const UISegmentedControlStyleBezeled: UISegmentedControlStyle = 3;

const CORNER_RADIUS: f32 = 8.0;
const DIVIDER_WIDTH: f32 = 1.0;
const FONT_SIZE: f32 = 13.0;

// MARK: - UISegment host object

#[derive(Default)]
struct UISegmentHostObject {
    title: id,
    image: id,
}
impl HostObject for UISegmentHostObject {}

// MARK: - UISegmentedControl host object

pub struct UISegmentedControlHostObject {
    pub(super) superclass: super::UIControlHostObject,
    segments: id,
    selected_index: NSInteger,
    momentary: bool,
    style: UISegmentedControlStyle,
    labels: id,
    dividers: id,
    selection_view: id,
    corner_filler: id,
    visuals_initialized: bool,
}
impl_HostObject_with_superclass!(UISegmentedControlHostObject);

impl Default for UISegmentedControlHostObject {
    fn default() -> Self {
        UISegmentedControlHostObject {
            superclass: Default::default(),
            segments: nil,
            selected_index: 0,
            momentary: false,
            style: UISegmentedControlStylePlain,
            labels: nil,
            dividers: nil,
            selection_view: nil,
            corner_filler: nil,
            visuals_initialized: false,
        }
    }
}

// MARK: - Layout helper

fn layout(env: &mut Environment, this: id) {
    let &UISegmentedControlHostObject {
        segments,
        selected_index,
        labels,
        dividers,
        selection_view,
        corner_filler,
        ..
    } = env.objc.borrow(this);

    if segments == nil || labels == nil {
        return;
    }

    let bounds: CGRect = msg![env; this bounds];
    let count: u32 = msg![env; segments count];
    if count == 0 {
        return;
    }

    let seg_w = bounds.size.width / count as f32;
    let h = bounds.size.height;

    // Selection view
    if selection_view != nil {
        let sel_x = if selected_index >= 0 && (selected_index as u32) < count {
            bounds.origin.x + selected_index as f32 * seg_w
        } else {
            -seg_w
        };
        let sel_rect = CGRect {
            origin: CGPoint {
                x: sel_x,
                y: bounds.origin.y,
            },
            size: CGSize {
                width: seg_w,
                height: h,
            },
        };
        () = msg![env; selection_view setFrame:sel_rect];

        let is_first = selected_index == 0;
        let is_last = selected_index == (count as NSInteger - 1);
        let layer: id = msg![env; selection_view layer];
        let r: CGFloat = if is_first || is_last {
            CORNER_RADIUS as CGFloat
        } else {
            0.0
        };
        () = msg![env; layer setCornerRadius:r];

        if corner_filler != nil {
            let hidden = selected_index < 0 || (selected_index as u32) >= count;
            () = msg![env; corner_filler setHidden:hidden];

            if !hidden {
                let filler_rect = if is_first {
                    CGRect {
                        origin: CGPoint {
                            x: sel_x + seg_w - CORNER_RADIUS,
                            y: bounds.origin.y,
                        },
                        size: CGSize {
                            width: CORNER_RADIUS,
                            height: h,
                        },
                    }
                } else if is_last {
                    CGRect {
                        origin: CGPoint {
                            x: sel_x,
                            y: bounds.origin.y,
                        },
                        size: CGSize {
                            width: CORNER_RADIUS,
                            height: h,
                        },
                    }
                } else {
                    () = msg![env; corner_filler setHidden:true];
                    CGRect {
                        origin: CGPoint { x: 0.0, y: 0.0 },
                        size: CGSize {
                            width: 0.0,
                            height: 0.0,
                        },
                    }
                };
                () = msg![env; corner_filler setFrame:filler_rect];
            }
        }
    }

    // Content Views (Labels/Images)
    let label_count: u32 = msg![env; labels count];
    let label_class = env.objc.get_known_class("UILabel", &mut env.mem);

    for i in 0..label_count {
        let content_view: id = msg![env; labels objectAtIndex:i];
        let content_rect = CGRect {
            origin: CGPoint {
                x: bounds.origin.x + i as f32 * seg_w,
                y: bounds.origin.y,
            },
            size: CGSize {
                width: seg_w,
                height: h,
            },
        };
        () = msg![env; content_view setFrame:content_rect];

        let subviews: id = msg![env; content_view subviews];
        let sub_c: u32 = if subviews != nil {
            msg![env; subviews count]
        } else {
            0
        };

        for j in 0..sub_c {
            let sub: id = msg![env; subviews objectAtIndex:j];
            let sub_rect = CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: content_rect.size,
            };
            () = msg![env; sub setFrame:sub_rect];

            let is_label: bool = msg![env; sub isKindOfClass:label_class];
            if is_label {
                let is_selected = i as NSInteger == selected_index;
                let color: id = if is_selected {
                    msg_class![env; UIColor whiteColor]
                } else {
                    msg_class![env; UIColor colorWithRed:(0.2f32) green:(0.2f32) blue:(0.2f32) alpha:1.0f32]
                };
                () = msg![env; sub setTextColor:color];
            }
        }
    }

    // Dividers
    if dividers == nil {
        return;
    }
    let div_count: u32 = msg![env; dividers count];
    for i in 0..div_count {
        let div: id = msg![env; dividers objectAtIndex:i];
        let left_of_sel = selected_index >= 0 && i as NSInteger == selected_index - 1;
        let right_of_sel = i as NSInteger == selected_index;
        () = msg![env; div setHidden:(left_of_sel || right_of_sel)];
        let div_x = bounds.origin.x + (i + 1) as f32 * seg_w - DIVIDER_WIDTH / 2.0;
        let div_rect = CGRect {
            origin: CGPoint {
                x: div_x,
                y: bounds.origin.y + 4.0,
            },
            size: CGSize {
                width: DIVIDER_WIDTH,
                height: h - 8.0,
            },
        };
        () = msg![env; div setFrame:div_rect];
    }
}

// MARK: - Visual init helper

fn init_visuals(env: &mut Environment, this: id) {
    if env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .visuals_initialized
    {
        return;
    }
    env.objc
        .borrow_mut::<UISegmentedControlHostObject>(this)
        .visuals_initialized = true;

    let font_size: CGFloat = FONT_SIZE as CGFloat;
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];

    let bg_color: id =
        msg_class![env; UIColor colorWithRed:(0.90f32) green:(0.90f32) blue:(0.90f32) alpha:1.0f32];
    () = msg![env; this setBackgroundColor:bg_color];
    {
        let layer: id = msg![env; this layer];
        let r: CGFloat = CORNER_RADIUS as CGFloat;
        () = msg![env; layer setCornerRadius:r];
        let border_color: id = msg_class![env; UIColor colorWithRed:(0.4f32) green:(0.4f32) blue:(0.4f32) alpha:(0.5f32)];
        let cg: id = msg![env; border_color CGColor];
        () = msg![env; layer setBorderColor:cg];
        let bw: CGFloat = 0.5;
        () = msg![env; layer setBorderWidth:bw];
    }

    let sel_color: id =
        msg_class![env; UIColor colorWithRed:(0.0f32) green:(0.478f32) blue:(1.0f32) alpha:1.0f32];
    let selection_view: id = msg_class![env; UIView new];
    () = msg![env; selection_view setBackgroundColor:sel_color];

    let corner_filler: id = msg_class![env; UIView new];
    () = msg![env; corner_filler setBackgroundColor:sel_color];

    let segments: id = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments;
    let count: u32 = if segments != nil {
        msg![env; segments count]
    } else {
        0
    };

    let labels: id = msg_class![env; NSMutableArray new];
    let dividers: id = msg_class![env; NSMutableArray new];

    let blue: id =
        msg_class![env; UIColor colorWithRed:(0.2f32) green:(0.2f32) blue:(0.2f32) alpha:1.0f32];
    let clear: id = msg_class![env; UIColor clearColor];

    let string_class = env.objc.get_known_class("NSString", &mut env.mem);
    let image_class = env.objc.get_known_class("UIImage", &mut env.mem);
    let segment_class = env.objc.get_known_class("UISegment", &mut env.mem);

    // Универсальная система извлечения данных из NIB-массивов
    for i in 0..count {
        let item: id = msg![env; segments objectAtIndex:i];
        let mut actual_text: id = nil;
        let mut actual_image: id = nil;

        if item != nil {
            if msg![env; item isKindOfClass:string_class] {
                actual_text = item;
            } else if msg![env; item isKindOfClass:image_class] {
                actual_image = item;
            } else if msg![env; item isKindOfClass:segment_class] {
                let sel_title = env.objc.lookup_selector("title").unwrap();
                if msg![env; item respondsToSelector:sel_title] {
                    actual_text = msg![env; item title];
                }
                let sel_image = env.objc.lookup_selector("image").unwrap();
                if msg![env; item respondsToSelector:sel_image] {
                    actual_image = msg![env; item image];
                }
            } else {
                let sel_t = env.objc.lookup_selector("title").unwrap();
                if msg![env; item respondsToSelector:sel_t] {
                    actual_text = msg![env; item title];
                }
            }
        }

        let content_view: id = msg_class![env; UIView new];
        () = msg![env; content_view setBackgroundColor:clear];

        if actual_image != nil {
            let img_view: id = msg_class![env; UIImageView new];
            () = msg![env; img_view setImage:actual_image];
            let content_mode: NSInteger = 4; // UIViewContentModeCenter
            () = msg![env; img_view setContentMode:content_mode];
            () = msg![env; img_view setBackgroundColor:clear];
            () = msg![env; content_view addSubview:img_view];
            release(env, img_view);
        }

        // Создаем UILabel в любом случае, чтобы layout не ломался
        let final_text = if actual_text != nil {
            actual_text
        } else {
            get_static_str(env, "")
        };
        let label: id = msg_class![env; UILabel new];
        () = msg![env; label setBackgroundColor:clear];
        () = msg![env; label setTextAlignment:UITextAlignmentCenter];
        () = msg![env; label setTextColor:blue];
        () = msg![env; label setFont:font];
        () = msg![env; label setText:final_text];
        () = msg![env; content_view addSubview:label];
        release(env, label);

        () = msg![env; labels addObject:content_view];
        release(env, content_view);
    }

    let div_color: id =
        msg_class![env; UIColor colorWithRed:(0.4f32) green:(0.4f32) blue:(0.4f32) alpha:0.6f32];
    for _ in 0..count.saturating_sub(1) {
        let div: id = msg_class![env; UIView new];
        () = msg![env; div setBackgroundColor:div_color];
        () = msg![env; dividers addObject:div];
        release(env, div);
    }

    {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
        host.selection_view = selection_view;
        host.corner_filler = corner_filler;
        host.labels = labels;
        host.dividers = dividers;
    }

    () = msg![env; this addSubview:selection_view];
    () = msg![env; this addSubview:corner_filler];
    let div_count: u32 = msg![env; dividers count];
    for i in 0..div_count {
        let div: id = msg![env; dividers objectAtIndex:i];
        () = msg![env; this addSubview:div];
    }
    let label_count: u32 = msg![env; labels count];
    for i in 0..label_count {
        let label: id = msg![env; labels objectAtIndex:i];
        () = msg![env; this addSubview:label];
    }

    layout(env, this);
}

// MARK: - Classes

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISegmentedControl: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UISegmentedControlHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithItems:(id)items {
    let zero = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size:   CGSize  { width: 0.0, height: 0.0 },
    };
    let this: id = msg_super![env; this initWithFrame:zero];

    let segments: id = msg_class![env; NSMutableArray new];
    let count: u32 = if items != nil { msg![env; items count] } else { 0 };
    for i in 0..count {
        let item: id = msg![env; items objectAtIndex:i];
        retain(env, item);
        () = msg![env; segments addObject:item];
    }
    env.objc.borrow_mut::<UISegmentedControlHostObject>(this).segments = segments;

    init_visuals(env, this);
    this
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    let segments: id = env.objc.borrow::<UISegmentedControlHostObject>(this).segments;
    if segments == nil {
        let segs: id = msg_class![env; NSMutableArray new];
        env.objc.borrow_mut::<UISegmentedControlHostObject>(this).segments = segs;
    }
    init_visuals(env, this);
    this
}

- (())setFrame:(CGRect)frame {
    () = msg_super![env; this setFrame:frame];
    layout(env, this);
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    let segments: id = msg_class![env; NSMutableArray new];
    env.objc.borrow_mut::<UISegmentedControlHostObject>(this).segments = segments;

    let key = get_static_str(env, "UISegments");
    if msg![env; coder containsValueForKey:key] {
        let seg_array: id = msg![env; coder decodeObjectForKey:key];
        if seg_array != nil {
            let count: u32 = msg![env; seg_array count];
            for i in 0..count {
                let item: id = msg![env; seg_array objectAtIndex:i];
                retain(env, item);
                // Сохраняем "сырой" объект, init_visuals сам разберется, строка
                // это или UISegment
                () = msg![env; segments addObject:item];
            }
        }
    }

    let key_index = get_static_str(env, "UISelectedSegmentIndex");
    if msg![env; coder containsValueForKey:key_index] {
        let idx: i32 = msg![env; coder decodeIntForKey:key_index];
        env.objc.borrow_mut::<UISegmentedControlHostObject>(this).selected_index = idx as NSInteger;
    }

    let key_style = get_static_str(env, "UISegmentedControlStyle");
    if msg![env; coder containsValueForKey:key_style] {
        let style: i32 = msg![env; coder decodeIntForKey:key_style];
        env.objc.borrow_mut::<UISegmentedControlHostObject>(this).style = style as NSInteger;
    }

    init_visuals(env, this);
    this
}

- (())dealloc {
    let &UISegmentedControlHostObject {
        segments, labels, dividers, selection_view, corner_filler, ..
    } = env.objc.borrow(this);
    release(env, segments);
    release(env, labels);
    release(env, dividers);
    release(env, selection_view);
    release(env, corner_filler);
    msg_super![env; this dealloc]
}

- (())layoutSubviews {
    layout(env, this);
}

- (NSInteger)numberOfSegments {
    let segments = env.objc.borrow::<UISegmentedControlHostObject>(this).segments;
    if segments == nil { return 0; }
    let count: u32 = msg![env; segments count];
    count as NSInteger
}

- (())insertSegmentWithTitle:(id)title atIndex:(NSInteger)index animated:(bool)_animated {
    let font_size: CGFloat = FONT_SIZE as CGFloat;
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
    let blue: id = msg_class![env; UIColor colorWithRed:(0.2f32) green:(0.2f32) blue:(0.2f32) alpha:1.0f32];
    let clear: id = msg_class![env; UIColor clearColor];

    let (segments, labels, dividers) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.labels, h.dividers)
    };
    if segments == nil { return; }
    let count: u32 = msg![env; segments count];
    let idx = (index as u32).min(count);
    retain(env, title);
    () = msg![env; segments insertObject:title atIndex:idx];

    let content_view: id = msg_class![env; UIView new];
    () = msg![env; content_view setBackgroundColor:clear];

    let label: id = msg_class![env; UILabel new];
    () = msg![env; label setBackgroundColor:clear];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setTextColor:blue];
    () = msg![env; label setFont:font];
    let actual_title = if title != nil { title } else { get_static_str(env, "") };
    () = msg![env; label setText:actual_title];
    () = msg![env; content_view addSubview:label];
    release(env, label);

    () = msg![env; labels insertObject:content_view atIndex:idx];
    () = msg![env; this addSubview:content_view];
    release(env, content_view);

    let new_count: u32 = msg![env; segments count];
    if new_count > 1 {
        let div_color: id = msg_class![env; UIColor colorWithRed:(0.55f32) green:(0.60f32) blue:(0.70f32) alpha:1.0f32];
        let div: id = msg_class![env; UIView new];
        () = msg![env; div setBackgroundColor:div_color];
        () = msg![env; dividers addObject:div];
        () = msg![env; this addSubview:div];
        release(env, div);
    }
    layout(env, this);
}

- (())insertSegmentWithImage:(id)image atIndex:(NSInteger)index animated:(bool)_animated {
    let font_size: CGFloat = FONT_SIZE as CGFloat;
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:font_size];
    let clear: id = msg_class![env; UIColor clearColor];
    let blue: id = msg_class![env; UIColor colorWithRed:(0.2f32) green:(0.2f32) blue:(0.2f32) alpha:1.0f32];

    let (segments, labels, dividers) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.labels, h.dividers)
    };
    if segments == nil { return; }
    let count: u32 = msg![env; segments count];
    let idx = (index as u32).min(count);
    retain(env, image);
    () = msg![env; segments insertObject:image atIndex:idx];

    let content_view: id = msg_class![env; UIView new];
    () = msg![env; content_view setBackgroundColor:clear];

    let img_view: id = msg_class![env; UIImageView new];
    () = msg![env; img_view setImage:image];
    let content_mode: NSInteger = 4;
    () = msg![env; img_view setContentMode:content_mode];
    () = msg![env; img_view setBackgroundColor:clear];
    () = msg![env; content_view addSubview:img_view];
    release(env, img_view);

    let empty = get_static_str(env, "");
    let label: id = msg_class![env; UILabel new];
    () = msg![env; label setBackgroundColor:clear];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setTextColor:blue];
    () = msg![env; label setFont:font];
    () = msg![env; label setText:empty];
    () = msg![env; content_view addSubview:label];
    release(env, label);

    () = msg![env; labels insertObject:content_view atIndex:idx];
    () = msg![env; this addSubview:content_view];
    release(env, content_view);

    let new_count: u32 = msg![env; segments count];
    if new_count > 1 {
        let div_color: id = msg_class![env; UIColor colorWithRed:(0.55f32) green:(0.60f32) blue:(0.70f32) alpha:1.0f32];
        let div: id = msg_class![env; UIView new];
        () = msg![env; div setBackgroundColor:div_color];
        () = msg![env; dividers addObject:div];
        () = msg![env; this addSubview:div];
        release(env, div);
    }
    layout(env, this);
}

- (())removeSegmentAtIndex:(NSInteger)index animated:(bool)_animated {
    let (segments, labels, dividers) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.labels, h.dividers)
    };
    if segments == nil { return; }
    let count: u32 = msg![env; segments count];
    if index < 0 || index as u32 >= count { return; }
    () = msg![env; segments removeObjectAtIndex:(index as u32)];
    let view: id = msg![env; labels objectAtIndex:(index as u32)];
    () = msg![env; view removeFromSuperview];
    () = msg![env; labels removeObjectAtIndex:(index as u32)];
    let div_count: u32 = msg![env; dividers count];
    if div_count > 0 {
        let last: id = msg![env; dividers objectAtIndex:(div_count - 1)];
        () = msg![env; last removeFromSuperview];
        () = msg![env; dividers removeObjectAtIndex:(div_count - 1)];
    }
    layout(env, this);
}

- (())removeAllSegments {
    let (segments, labels, dividers) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.labels, h.dividers)
    };
    if segments == nil { return; }
    let lc: u32 = msg![env; labels   count];
    let dc: u32 = msg![env; dividers count];
    for i in 0..lc { let v: id = msg![env; labels objectAtIndex:i]; () = msg![env; v removeFromSuperview]; }
    for i in 0..dc { let v: id = msg![env; dividers objectAtIndex:i]; () = msg![env; v removeFromSuperview]; }
    () = msg![env; segments removeAllObjects];
    () = msg![env; labels removeAllObjects];
    () = msg![env; dividers removeAllObjects];
    layout(env, this);
}

- (())setTitle:(id)title forSegmentAtIndex:(NSInteger)index {
    let (segments, labels) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.labels)
    };
    if segments == nil { return; }
    let count: u32 = msg![env; segments count];
    if index < 0 || index as u32 >= count { return; }
    retain(env, title);
    () = msg![env; segments replaceObjectAtIndex:(index as u32) withObject:title];

    let content_view: id = msg![env; labels objectAtIndex:(index as u32)];
    let subviews: id = msg![env; content_view subviews];
    let sub_c: u32 = if subviews != nil { msg![env; subviews count] } else { 0 };
    let label_class = env.objc.get_known_class("UILabel", &mut env.mem);

    for j in 0..sub_c {
        let sub: id = msg![env; subviews objectAtIndex:j];
        if msg![env; sub isKindOfClass:label_class] {
            () = msg![env; sub setText:title];
        }
    }
}

- (id)titleForSegmentAtIndex:(NSInteger)index {
    let segments = env.objc.borrow::<UISegmentedControlHostObject>(this).segments;
    if segments == nil { return nil; }
    let count: u32 = msg![env; segments count];
    if index < 0 || index as u32 >= count { return nil; }
    msg![env; segments objectAtIndex:(index as u32)]
}

- (())setImage:(id)image forSegmentAtIndex:(NSInteger)index {
    let segments = env.objc.borrow::<UISegmentedControlHostObject>(this).segments;
    if segments == nil { return; }
    let count: u32 = msg![env; segments count];
    if index < 0 || index as u32 >= count { return; }
    retain(env, image);
    () = msg![env; segments replaceObjectAtIndex:(index as u32) withObject:image];
}

- (id)imageForSegmentAtIndex:(NSInteger)index {
    let segments = env.objc.borrow::<UISegmentedControlHostObject>(this).segments;
    if segments == nil { return nil; }
    let count: u32 = msg![env; segments count];
    if index < 0 || index as u32 >= count { return nil; }
    msg![env; segments objectAtIndex:(index as u32)]
}

// MARK: - Selection

- (NSInteger)selectedSegmentIndex {
    env.objc.borrow::<UISegmentedControlHostObject>(this).selected_index
}

- (())setSelectedSegmentIndex:(NSInteger)index {
    env.objc.borrow_mut::<UISegmentedControlHostObject>(this).selected_index = index;
    layout(env, this);
}

// MARK: - Style

- (UISegmentedControlStyle)segmentedControlStyle {
    env.objc.borrow::<UISegmentedControlHostObject>(this).style
}

- (())setSegmentedControlStyle:(UISegmentedControlStyle)style {
    env.objc.borrow_mut::<UISegmentedControlHostObject>(this).style = style;
}

- (bool)isMomentary {
    env.objc.borrow::<UISegmentedControlHostObject>(this).momentary
}
- (())setMomentary:(bool)value {
    env.objc.borrow_mut::<UISegmentedControlHostObject>(this).momentary = value;
}

// MARK: - Enable/disable

- (())setEnabled:(bool)_enabled forSegmentAtIndex:(NSInteger)_index {}
- (bool)isEnabledForSegmentAtIndex:(NSInteger)_index { true }

// MARK: - Width / offset stubs

- (())setWidth:(CGFloat)_w forSegmentAtIndex:(NSInteger)_i {}
- (CGFloat)widthForSegmentAtIndex:(NSInteger)_i { 0.0 }
- (())setContentOffset:(CGSize)_o forSegmentAtIndex:(NSInteger)_i {}

// MARK: - Touch handling

- (bool)beginTrackingWithTouch:(id)touch withEvent:(id)event {
    let touch_point: CGPoint = msg![env; touch locationInView:this];
    let bounds: CGRect = msg![env; this bounds];

    let (segments, momentary, old_index) = {
        let h = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (h.segments, h.momentary, h.selected_index)
    };
    if segments == nil { return false; }
    let count: u32 = msg![env; segments count];
    if count == 0 { return false; }

    let seg_w = bounds.size.width / count as f32;
    let tapped = ((touch_point.x - bounds.origin.x) / seg_w) as NSInteger;
    let tapped = tapped.max(0).min(count as NSInteger - 1);

    if momentary || tapped != old_index {
        env.objc.borrow_mut::<UISegmentedControlHostObject>(this).selected_index = tapped;
        layout(env, this);
        send_actions(env, this, event, UIControlEventValueChanged);
    }
    true
}

- (bool)continueTrackingWithTouch:(id)_touch withEvent:(id)_event { true }

- (())endTrackingWithTouch:(id)touch withEvent:(id)event {
    let momentary = env.objc.borrow::<UISegmentedControlHostObject>(this).momentary;
    if momentary {
        env.objc.borrow_mut::<UISegmentedControlHostObject>(this).selected_index = -1;
        layout(env, this);
    }
    msg_super![env; this endTrackingWithTouch:touch withEvent:event]
}

// MARK: - Hit testing

- (id)hitTest:(CGPoint)point withEvent:(id)event {
    if msg![env; this pointInside:point withEvent:event] { this } else { nil }
}

// MARK: - Appearance stubs (iOS 5+)

- (())setBackgroundImage:(id)_i forState:(NSUInteger)_s barMetrics:(NSInteger)_m {}
- (())setDividerImage:(id)_i forLeftSegmentState:(NSUInteger)_l rightSegmentState:(NSUInteger)_r barMetrics:(NSInteger)_m {}
- (())setTitleTextAttributes:(id)_a forState:(NSUInteger)_s {}
- (())setContentPositionAdjustment:(CGSize)_a forSegmentType:(NSInteger)_t barMetrics:(NSInteger)_m {}

@end

// MARK: - UISegment (undocumented internal class used by NIB decoder)

@implementation UISegment: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UISegmentHostObject { title: nil, image: nil });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    let key_title = get_static_str(env, "UITitle");
    let key_label = get_static_str(env, "UILabel");

    let mut title: id = nil;
    if msg![env; coder containsValueForKey:key_title] {
        title = msg![env; coder decodeObjectForKey:key_title];
    } else if msg![env; coder containsValueForKey:key_label] {
        title = msg![env; coder decodeObjectForKey:key_label];
    }

    if title != nil {
        let label_class = env.objc.get_known_class("UILabel", &mut env.mem);
        let is_label: bool = msg![env; title isKindOfClass:label_class];
        if is_label {
            title = msg![env; title text];
        }
    }
    retain(env, title);

    let key_img = get_static_str(env, "UIImage");
    let image: id = if msg![env; coder containsValueForKey:key_img] {
        msg![env; coder decodeObjectForKey:key_img]
    } else {
        nil
    };
    retain(env, image);

    let host = env.objc.borrow_mut::<UISegmentHostObject>(this);
    host.title = title;
    host.image = image;
    this
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (())dealloc {
    let &UISegmentHostObject { title, image } = env.objc.borrow(this);
    release(env, title);
    release(env, image);
    msg_super![env; this dealloc]
}

- (())setTitle:(id)title {
    let old = env.objc.borrow::<UISegmentHostObject>(this).title;
    release(env, old);
    retain(env, title);
    env.objc.borrow_mut::<UISegmentHostObject>(this).title = title;
}

- (id)title {
    env.objc.borrow::<UISegmentHostObject>(this).title
}

- (())setImage:(id)image {
    let old = env.objc.borrow::<UISegmentHostObject>(this).image;
    release(env, old);
    retain(env, image);
    env.objc.borrow_mut::<UISegmentHostObject>(this).image = image;
}

- (id)image {
    env.objc.borrow::<UISegmentHostObject>(this).image
}

@end

};
