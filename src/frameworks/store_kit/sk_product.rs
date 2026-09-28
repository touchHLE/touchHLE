/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::foundation::ns_array;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_super, nil, objc_classes, ClassExports, HostObject,
    NSZonePtr, SEL,
};

struct SKProductsRequestHostObject {
    delegate: id,
}
impl HostObject for SKProductsRequestHostObject {}

struct SKProductsResponseHostObject;
impl HostObject for SKProductsResponseHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKProduct: NSObject
// TODO
@end

@implementation SKProductsResponse: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(SKProductsResponseHostObject),
        &mut env.mem
    )
}

- (id)products {
    let products = ns_array::from_vec(env, Vec::new());
    autorelease(env, products)
}

- (id)invalidProductIdentifiers {
    let identifiers = ns_array::from_vec(env, Vec::new());
    autorelease(env, identifiers)
}

@end

@implementation SKProductsRequest: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(SKProductsRequestHostObject { delegate: nil }),
        &mut env.mem
    )
}

- (id)initWithProductIdentifiers:(id)_ids { // NSSet *
    log!("Ignoring StoreKit product request; returning no products.");
    msg_super![env; this init]
}

- (())setDelegate:(id)new_delegate {
    log!("SKProductsRequest delegate set to {:?}.", new_delegate);
    env.objc
        .borrow_mut::<SKProductsRequestHostObject>(this)
        .delegate = new_delegate;
}

- (id)delegate {
    env.objc
        .borrow::<SKProductsRequestHostObject>(this)
        .delegate
}

- (())start {
    let delegate = env
        .objc
        .borrow::<SKProductsRequestHostObject>(this)
        .delegate;
    log!("SKProductsRequest start; sending empty product response.");
    if delegate == nil {
        return;
    }

    let response: id = msg_class![env; SKProductsResponse alloc];
    let response: id = msg![env; response init];
    let selector: SEL = env.objc.register_host_selector(
        "productsRequest:didReceiveResponse:".to_string(),
        &mut env.mem,
    );
    if msg![env; delegate respondsToSelector:selector] {
        () = msg![env; delegate productsRequest:this didReceiveResponse:response];
    } else {
        let selector: SEL = env
            .objc
            .register_host_selector("request:didFailWithError:".to_string(), &mut env.mem);
        if msg![env; delegate respondsToSelector:selector] {
            () = msg![env; delegate request:this didFailWithError:nil];
        }
    }
}

- (())cancel {
    log!("Ignoring SKProductsRequest cancel.");
}
@end

};
