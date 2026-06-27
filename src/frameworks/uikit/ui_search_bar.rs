/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! UISearchBar.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{
    id, msg_super, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

#[derive(Default)]
struct UISearchBarHostObject {
    delegate: id,
    text: id,
    placeholder: id,
    prompt: id,
    bar_style: i32,
    search_bar_style: i32,
    shows_cancel_button: bool,
    shows_bookmark_button: bool,
    shows_search_results_button: bool,
    search_results_button_selected: bool,

    autocorrection_type: i32,
    autocapitalization_type: i32,
    keyboard_type: i32,
    return_key_type: i32,
    spell_checking_type: i32,
    enables_return_key_automatically: bool,

    tint_color: id,
    bar_tint_color: id,
    translucent: bool,
    input_accessory_view: id,
    input_view: id,
    scope_button_titles: id,
    selected_scope_button_index: i32,
    shows_scope_bar: bool,
    background_image: id,
    scope_bar_background_image: id,

    search_field_background_position_adjustment: id,
    search_text_position_adjustment: id,

    is_first_responder: bool,
}

impl HostObject for UISearchBarHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISearchBar: UIView

// Исправление паники "Call to class method alloc"
+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UISearchBarHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    // Безопасно извлекаем свойства для очистки памяти
    let (
        text, placeholder, prompt, tint_color, bar_tint_color,
        input_accessory_view, input_view, scope_button_titles,
        background_image, scope_bar_background_image,
        adj1, adj2
    ) = {
        let host = env.objc.borrow::<UISearchBarHostObject>(this);
        (
            host.text, host.placeholder, host.prompt, host.tint_color, host.bar_tint_color,
            host.input_accessory_view, host.input_view, host.scope_button_titles,
            host.background_image, host.scope_bar_background_image,
            host.search_field_background_position_adjustment, host.search_text_position_adjustment
        )
    };

    release(env, text);
    release(env, placeholder);
    release(env, prompt);
    release(env, tint_color);
    release(env, bar_tint_color);
    release(env, input_accessory_view);
    release(env, input_view);
    release(env, scope_button_titles);
    release(env, background_image);
    release(env, scope_bar_background_image);
    release(env, adj1);
    release(env, adj2);

    msg_super![env; this dealloc]
}

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

// Исправление для NIB-парсера (UIClassSwapper)
- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (id)delegate {
    env.objc.borrow::<UISearchBarHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    // Делегаты в UIKit обычно имеют weak-ссылку, поэтому без retain/release
    env.objc.borrow_mut::<UISearchBarHostObject>(this).delegate = delegate;
}

- (id)text {
    env.objc.borrow::<UISearchBarHostObject>(this).text
}
- (())setText:(id)text {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).text;
    retain(env, text);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).text = text;
}

- (id)placeholder {
    env.objc.borrow::<UISearchBarHostObject>(this).placeholder
}
- (())setPlaceholder:(id)placeholder {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).placeholder;
    retain(env, placeholder);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).placeholder = placeholder;
}

- (id)prompt {
    env.objc.borrow::<UISearchBarHostObject>(this).prompt
}
- (())setPrompt:(id)prompt {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).prompt;
    retain(env, prompt);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).prompt = prompt;
}

- (())setBarStyle:(i32)style {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).bar_style = style;
}
- (i32)barStyle {
    env.objc.borrow::<UISearchBarHostObject>(this).bar_style
}

- (())setSearchBarStyle:(i32)style {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).search_bar_style = style;
}
- (i32)searchBarStyle {
    env.objc.borrow::<UISearchBarHostObject>(this).search_bar_style
}

- (())setShowsCancelButton:(bool)shows {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).shows_cancel_button = shows;
}
- (())setShowsCancelButton:(bool)shows animated:(bool)_animated {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).shows_cancel_button = shows;
}
- (bool)showsCancelButton {
    env.objc.borrow::<UISearchBarHostObject>(this).shows_cancel_button
}

- (())setShowsBookmarkButton:(bool)shows {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).shows_bookmark_button = shows;
}
- (bool)showsBookmarkButton {
    env.objc.borrow::<UISearchBarHostObject>(this).shows_bookmark_button
}

- (())setShowsSearchResultsButton:(bool)shows {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).shows_search_results_button = shows;
}
- (bool)showsSearchResultsButton {
    env.objc.borrow::<UISearchBarHostObject>(this).shows_search_results_button
}

- (())setSearchResultsButtonSelected:(bool)selected {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).search_results_button_selected = selected;
}
- (bool)isSearchResultsButtonSelected {
    env.objc.borrow::<UISearchBarHostObject>(this).search_results_button_selected
}

- (())setAutocorrectionType:(i32)type_val {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).autocorrection_type = type_val;
}
- (())setAutocapitalizationType:(i32)type_val {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).autocapitalization_type = type_val;
}
- (())setKeyboardType:(i32)type_val {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).keyboard_type = type_val;
}
- (())setReturnKeyType:(i32)type_val {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).return_key_type = type_val;
}
- (())setSpellCheckingType:(i32)type_val {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).spell_checking_type = type_val;
}
- (())setEnablesReturnKeyAutomatically:(bool)enables {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).enables_return_key_automatically = enables;
}

- (())setTintColor:(id)color {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).tint_color;
    retain(env, color);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).tint_color = color;
}
- (id)tintColor {
    env.objc.borrow::<UISearchBarHostObject>(this).tint_color
}

- (())setBarTintColor:(id)color {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).bar_tint_color;
    retain(env, color);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).bar_tint_color = color;
}
- (id)barTintColor {
    env.objc.borrow::<UISearchBarHostObject>(this).bar_tint_color
}

- (())setTranslucent:(bool)translucent {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).translucent = translucent;
}
- (bool)isTranslucent {
    env.objc.borrow::<UISearchBarHostObject>(this).translucent
}

- (())setInputAccessoryView:(id)view {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).input_accessory_view;
    retain(env, view);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).input_accessory_view = view;
}
- (id)inputAccessoryView {
    env.objc.borrow::<UISearchBarHostObject>(this).input_accessory_view
}

- (())setInputView:(id)view {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).input_view;
    retain(env, view);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).input_view = view;
}
- (id)inputView {
    env.objc.borrow::<UISearchBarHostObject>(this).input_view
}

- (())setScopeButtonTitles:(id)titles {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).scope_button_titles;
    retain(env, titles);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).scope_button_titles = titles;
}
- (id)scopeButtonTitles {
    env.objc.borrow::<UISearchBarHostObject>(this).scope_button_titles
}

- (())setSelectedScopeButtonIndex:(i32)index {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).selected_scope_button_index = index;
}
- (i32)selectedScopeButtonIndex {
    env.objc.borrow::<UISearchBarHostObject>(this).selected_scope_button_index
}

- (())setShowsScopeBar:(bool)shows {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).shows_scope_bar = shows;
}
- (bool)showsScopeBar {
    env.objc.borrow::<UISearchBarHostObject>(this).shows_scope_bar
}

- (())setBackgroundImage:(id)image {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).background_image;
    retain(env, image);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).background_image = image;
}
- (id)backgroundImage {
    env.objc.borrow::<UISearchBarHostObject>(this).background_image
}

- (())setScopeBarBackgroundImage:(id)image {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).scope_bar_background_image;
    retain(env, image);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).scope_bar_background_image = image;
}

- (())setSearchFieldBackgroundPositionAdjustment:(id)offset {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).search_field_background_position_adjustment;
    retain(env, offset);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).search_field_background_position_adjustment = offset;
}
- (())setSearchTextPositionAdjustment:(id)offset {
    let old = env.objc.borrow::<UISearchBarHostObject>(this).search_text_position_adjustment;
    retain(env, offset);
    release(env, old);
    env.objc.borrow_mut::<UISearchBarHostObject>(this).search_text_position_adjustment = offset;
}

- (bool)becomeFirstResponder {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).is_first_responder = true;
    true
}
- (bool)resignFirstResponder {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).is_first_responder = false;
    true
}
- (bool)isFirstResponder {
    env.objc.borrow::<UISearchBarHostObject>(this).is_first_responder
}

// Complex state UI modifiers are still logged to track app behavior
- (())setSearchFieldBackgroundImage:(id)_image forState:(i32)_state {
    log!("TODO: UISearchBar setSearchFieldBackgroundImage:forState:");
}
- (())setImage:(id)_image forSearchBarIcon:(i32)_icon state:(i32)_state {
    log!("TODO: UISearchBar setImage:forSearchBarIcon:state:");
}
- (())setScopeBarButtonBackgroundImage:(id)_image forState:(i32)_state {
    log!("TODO: UISearchBar setScopeBarButtonBackgroundImage:forState:");
}
- (())setScopeBarButtonDividerImage:(id)_image forLeftSegmentState:(i32)_left rightSegmentState:(i32)_right {
    log!("TODO: UISearchBar setScopeBarButtonDividerImage:forLeftSegmentState:rightSegmentState:");
}
- (())setScopeBarButtonTitleTextAttributes:(id)_attributes forState:(i32)_state {
    log!("TODO: UISearchBar setScopeBarButtonTitleTextAttributes:forState:");
}
- (())setPositionAdjustment:(id)_offset forSearchBarIcon:(i32)_icon {
    log!("TODO: UISearchBar setPositionAdjustment:forSearchBarIcon:");
}

@end

};
