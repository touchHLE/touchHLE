/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ABPeoplePickerNavigationController` and related AddressBook UI types.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

// MARK: - ABPersonSortOrdering / ABPersonCompositeNameFormat constants

pub type ABPersonSortOrdering = u32;
pub const AB_PERSON_SORT_BY_FIRST_NAME: ABPersonSortOrdering = 0;
pub const AB_PERSON_SORT_BY_LAST_NAME: ABPersonSortOrdering = 1;

pub type ABPersonCompositeNameFormat = u32;
pub const AB_PERSON_COMPOSITE_NAME_FORMAT_FIRST_NAME_FIRST: ABPersonCompositeNameFormat = 0;
pub const AB_PERSON_COMPOSITE_NAME_FORMAT_LAST_NAME_FIRST: ABPersonCompositeNameFormat = 1;

// MARK: - Host object

#[derive(Default)]
struct ABPeoplePickerNavigationControllerHostObject {
    /// Weak delegate reference (per Apple convention for picker delegates).
    people_picker_delegate: id,
    /// `NSArray*` of `NSNumber*` — ABPropertyID values to display.
    displayed_properties: id,
    /// `ABAddressBook*` — address book to browse (nil = default).
    address_book: id,
    /// `NSPredicate*` — used to filter people (iOS 8+, stub).
    predicate_for_enabling_selection: id,
    /// `NSPredicate*` — used to filter sections (iOS 8+, stub).
    predicate_for_section_header_selection: id,
    /// `NSPredicate*` — filter displayed people (iOS 8+, stub).
    predicate_for_displayed_results: id,
}
impl HostObject for ABPeoplePickerNavigationControllerHostObject {}

struct EmptyHostObject;
impl HostObject for EmptyHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation ABPeoplePickerNavigationController: UINavigationController

// =========================================================================
// MARK: - Allocation
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(ABPeoplePickerNavigationControllerHostObject {
        people_picker_delegate: nil,
        displayed_properties: nil,
        address_book: nil,
        predicate_for_enabling_selection: nil,
        predicate_for_section_header_selection: nil,
        predicate_for_displayed_results: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this);
    let (delegate, displayed, ab, p1, p2, p3) = (
        host.people_picker_delegate,
        host.displayed_properties,
        host.address_book,
        host.predicate_for_enabling_selection,
        host.predicate_for_section_header_selection,
        host.predicate_for_displayed_results,
    );
    release(env, delegate);
    release(env, displayed);
    release(env, ab);
    release(env, p1);
    release(env, p2);
    release(env, p3);
    env.objc.dealloc_object(this, &mut env.mem)
}

// =========================================================================
// MARK: - Delegate
// =========================================================================

- (id)peoplePickerDelegate {
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .people_picker_delegate
}

- (())setPeoplePickerDelegate:(id)delegate {
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .people_picker_delegate;
    release(env, old);
    retain(env, delegate);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .people_picker_delegate = delegate;
}

// =========================================================================
// MARK: - Displayed properties
// =========================================================================

- (id)displayedProperties { // NSArray* of NSNumber*
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .displayed_properties
}

- (())setDisplayedProperties:(id)properties { // NSArray*
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .displayed_properties;
    release(env, old);
    retain(env, properties);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .displayed_properties = properties;
}

// =========================================================================
// MARK: - Address book
// =========================================================================

- (id)addressBook {
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this).address_book
}

- (())setAddressBook:(id)address_book {
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .address_book;
    release(env, old);
    retain(env, address_book);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .address_book = address_book;
}

// =========================================================================
// MARK: - Predicates (iOS 8+ stubs)
// =========================================================================

- (id)predicateForEnablingSelection {
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_enabling_selection
}

- (())setPredicateForEnablingSelection:(id)predicate {
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_enabling_selection;
    release(env, old);
    retain(env, predicate);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_enabling_selection = predicate;
}

- (id)predicateForSectionHeaderSelection {
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_section_header_selection
}

- (())setPredicateForSectionHeaderSelection:(id)predicate {
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_section_header_selection;
    release(env, old);
    retain(env, predicate);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_section_header_selection = predicate;
}

- (id)predicateForDisplayedResults {
    env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_displayed_results
}

- (())setPredicateForDisplayedResults:(id)predicate {
    let old = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_displayed_results;
    release(env, old);
    retain(env, predicate);
    env.objc.borrow_mut::<ABPeoplePickerNavigationControllerHostObject>(this)
        .predicate_for_displayed_results = predicate;
}

// =========================================================================
// MARK: - UIViewController overrides
// touchHLE has no contacts UI. When presented we immediately fire
// peoplePickerNavigationControllerDidCancel: so the app can clean up.
// =========================================================================

- (())viewWillAppear:(bool)_animated {
    log!("ABPeoplePickerNavigationController: no contacts UI — cancelling immediately");

    let delegate = env.objc.borrow::<ABPeoplePickerNavigationControllerHostObject>(this)
        .people_picker_delegate;

    if delegate != nil {
        // iOS 8+ delegate method.
        let sel_cancel = env.objc
            .lookup_selector("peoplePickerNavigationControllerDidCancel:")
            .unwrap();
        let responds: bool = msg![env; delegate respondsToSelector:sel_cancel];
        if responds {
            let _: () = msg![env; delegate peoplePickerNavigationControllerDidCancel:this];
        }

        // Pre-iOS 8 delegate method (same name, same signature).
        // Already covered by the selector above on older SDKs.
    }
}

- (())viewDidAppear:(bool)animated {
    // Dismiss ourselves so the presenting controller is not left hanging.
    let presenting: id = msg![env; this presentingViewController];
    if presenting != nil {
        let _: () = msg![env; presenting dismissViewControllerAnimated:animated completion:nil];
    }
}

// =========================================================================
// MARK: - Description
// =========================================================================

- (id)description {
    let s = format!(
        "<ABPeoplePickerNavigationController: {:?} — no contacts support>",
        this
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =========================================================================
// MARK: - ABUnknownPersonViewController (stub)
// =========================================================================

@implementation ABUnknownPersonViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(EmptyHostObject), &mut env.mem)
}

- (id)init { this }

- (())setDisplayedPerson:(id)_person {
    log!("TODO: ABUnknownPersonViewController setDisplayedPerson: — ignored");
}

- (())setAlternateName:(id)name {
    log_dbg!(
        "ABUnknownPersonViewController setAlternateName: {:?}",
        if name != nil { ns_string::to_rust_string(env, name).into_owned() } else { "(null)".into() }
    );
}

- (())setTitle:(id)title {
    log_dbg!(
        "ABUnknownPersonViewController setTitle: {:?}",
        if title != nil { ns_string::to_rust_string(env, title).into_owned() } else { "(null)".into() }
    );
}

- (())setMessage:(id)message {
    log_dbg!(
        "ABUnknownPersonViewController setMessage: {:?}",
        if message != nil { ns_string::to_rust_string(env, message).into_owned() } else { "(null)".into() }
    );
}

- (())setAllowsAddingToAddressBook:(bool)allows {
    log_dbg!("ABUnknownPersonViewController setAllowsAddingToAddressBook:{}", allows);
}

- (())setAllowsActions:(bool)allows {
    log_dbg!("ABUnknownPersonViewController setAllowsActions:{}", allows);
}

- (id)unknownPersonViewDelegate { nil }
- (())setUnknownPersonViewDelegate:(id)_delegate {}

@end

// =========================================================================
// MARK: - ABPersonViewController (stub)
// =========================================================================

@implementation ABPersonViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(EmptyHostObject), &mut env.mem)
}

- (id)init { this }

- (())setDisplayedPerson:(id)_person {
    log!("TODO: ABPersonViewController setDisplayedPerson: — ignored");
}

- (())setAllowsEditing:(bool)allows {
    log_dbg!("ABPersonViewController setAllowsEditing:{}", allows);
}

- (())setAllowsActions:(bool)allows {
    log_dbg!("ABPersonViewController setAllowsActions:{}", allows);
}

- (())setDisplayedProperties:(id)_properties {
    log!("TODO: ABPersonViewController setDisplayedProperties: — ignored");
}

- (())setHighlightedItemForProperty:(i32)property withIdentifier:(i32)identifier {
    log_dbg!(
        "ABPersonViewController setHighlightedItemForProperty:{} withIdentifier:{}",
        property, identifier
    );
}

- (id)personViewDelegate { nil }
- (())setPersonViewDelegate:(id)_delegate {}

@end

// =========================================================================
// MARK: - ABNewPersonViewController (stub)
// =========================================================================

@implementation ABNewPersonViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(EmptyHostObject), &mut env.mem)
}

- (id)init { this }

- (())setDisplayedPerson:(id)_person {
    log!("TODO: ABNewPersonViewController setDisplayedPerson: — ignored");
}

- (())setAddressBook:(id)_ab {
    log!("TODO: ABNewPersonViewController setAddressBook: — ignored");
}

- (id)newPersonViewDelegate { nil }
- (())setNewPersonViewDelegate:(id)_delegate {}

- (())viewWillAppear:(bool)_animated {
    log!("ABNewPersonViewController: no contacts UI — cancelling immediately");
    let presenting: id = msg![env; this presentingViewController];
    if presenting != nil {
        let _: () = msg![env; presenting dismissViewControllerAnimated:false completion:nil];
    }
}

@end

};
