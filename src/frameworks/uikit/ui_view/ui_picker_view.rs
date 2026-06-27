/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPickerView`.

use crate::frameworks::foundation::NSUInteger;
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

// TODO: rendering

#[derive(Default)]
struct UIPickerViewHostObject {
    superclass: UIViewHostObject,
    delegate: id,
    data_source: id,
    shows_selection_indicator: bool,
    /// Cached component count (from data source).
    number_of_components: NSUInteger,
}
impl_HostObject_with_superclass!(UIPickerViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPickerView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPickerViewHostObject {
        superclass: UIViewHostObject::default(),
        delegate: nil,
        data_source: nil,
        shows_selection_indicator: false,
        number_of_components: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(id)frame {
    let _: () = msg![env; this setFrame:frame];
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UIPickerViewHostObject>(this);
    let (delegate, data_source) = (host.delegate, host.data_source);
    release(env, delegate);
    release(env, data_source);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Delegate

- (id)delegate {
    env.objc.borrow::<UIPickerViewHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let old = env.objc.borrow::<UIPickerViewHostObject>(this).delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<UIPickerViewHostObject>(this).delegate = delegate;
}

// MARK: - Data source

- (id)dataSource {
    env.objc.borrow::<UIPickerViewHostObject>(this).data_source
}

- (())setDataSource:(id)data_source {
    let old = env.objc.borrow::<UIPickerViewHostObject>(this).data_source;
    release(env, old);
    retain(env, data_source);
    env.objc.borrow_mut::<UIPickerViewHostObject>(this).data_source = data_source;
    // Refresh component count from the new data source.
    let count: NSUInteger = if data_source != nil {
        msg![env; data_source numberOfComponentsInPickerView:this]
    } else {
        0
    };
    env.objc.borrow_mut::<UIPickerViewHostObject>(this).number_of_components = count;
}

// MARK: - Selection indicator

- (bool)showsSelectionIndicator {
    env.objc.borrow::<UIPickerViewHostObject>(this).shows_selection_indicator
}

- (())setShowsSelectionIndicator:(bool)shows {
    env.objc.borrow_mut::<UIPickerViewHostObject>(this).shows_selection_indicator = shows;
}

// MARK: - Component / row counts

- (NSUInteger)numberOfComponents {
    env.objc.borrow::<UIPickerViewHostObject>(this).number_of_components
}

- (NSUInteger)numberOfRowsInComponent:(NSUInteger)component {
    let data_source = env.objc.borrow::<UIPickerViewHostObject>(this).data_source;
    if data_source == nil {
        return 0;
    }
    msg![env; data_source pickerView:this numberOfRowsInComponent:component]
}

// MARK: - Row size (delegate query)

- (id)rowSizeForComponent:(NSUInteger)component {
    let delegate = env.objc.borrow::<UIPickerViewHostObject>(this).delegate;
    if delegate != nil {
        let width:  f32 = msg![env; delegate pickerView:this widthForComponent:component];
        let height: f32 = msg![env; delegate pickerView:this rowHeightForComponent:component];
        if width > 0.0 && height > 0.0 {
            let size = crate::frameworks::core_graphics::CGSize { width, height };
            return msg_class![env; NSValue valueWithCGSize:size];
        }
    }
    let size = crate::frameworks::core_graphics::CGSize { width: 320.0, height: 44.0 };
    msg_class![env; NSValue valueWithCGSize:size]
}

// MARK: - Selection

- (NSUInteger)selectedRowInComponent:(NSUInteger)component {
    // Without a real model we always report row 0 as selected.
    log_dbg!("UIPickerView selectedRowInComponent:{} — returning 0 (stub)", component);
    0
}

- (())selectRow:(NSUInteger)row
    inComponent:(NSUInteger)component
        animated:(bool)animated {
    log_dbg!(
        "UIPickerView selectRow:{} inComponent:{} animated:{} — stub",
        row, component, animated
    );
    // Notify delegate of the selection.
    let delegate = env.objc.borrow::<UIPickerViewHostObject>(this).delegate;
    if delegate != nil {
        let _: () = msg![env; delegate pickerView:this
                                     didSelectRow:row
                                     inComponent:component];
    }
}

// MARK: - View for row (delegate query)

- (id)viewForRow:(NSUInteger)row
    forComponent:(NSUInteger)component {
    let delegate = env.objc.borrow::<UIPickerViewHostObject>(this).delegate;
    if delegate == nil {
        return nil;
    }
    // Ask delegate for a custom view; pass nil as the reusable view.
    msg![env; delegate pickerView:this
                      viewForRow:row
                    forComponent:component
                     reusingView:nil]
}

// MARK: - Reload

- (())reloadAllComponents {
    log_dbg!("UIPickerView reloadAllComponents");
    // Refresh component count.
    let data_source = env.objc.borrow::<UIPickerViewHostObject>(this).data_source;
    let count: NSUInteger = if data_source != nil {
        msg![env; data_source numberOfComponentsInPickerView:this]
    } else {
        0
    };
    env.objc.borrow_mut::<UIPickerViewHostObject>(this).number_of_components = count;
}

- (())reloadComponent:(NSUInteger)component {
    log_dbg!("UIPickerView reloadComponent:{} — stub", component);
    // No rendering model yet; just refresh total count.
    let _: () = msg![env; this reloadAllComponents];
}

@end

};
