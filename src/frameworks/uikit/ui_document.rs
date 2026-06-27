/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIDocument`.

use crate::abi::{CallFromHost, GuestFunction};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::ConstPtr; // Добавили ConstPtr
use crate::objc::{
    id, msg, msg_super, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

pub struct UIDocumentHostObject {
    file_url: id,
}

// Явная реализация трейта для устранения ошибки E0277
impl HostObject for UIDocumentHostObject {}

impl Default for UIDocumentHostObject {
    fn default() -> Self {
        Self { file_url: nil }
    }
}

// Вспомогательная функция для вызова Objective-C блоков
fn call_bool_block(env: &mut Environment, block: id, arg: bool) {
    if block != nil {
        let block_ptr = block.to_bits();
        // Явно указываем ConstPtr::<u32>, чтобы компилятор не гадал о
        // параметрах MUT и T
        let invoke_addr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(block_ptr + 12));
        let invoke_func = GuestFunction::from_addr_with_thumb_bit(invoke_addr);

        let _: () = invoke_func.call_from_host(env, (block, arg as u32));
    }
}

pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation UIDocument: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::<UIDocumentHostObject>::default();
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)initWithFileURL:(id)url {
        // Вместо msg![env; super init] используем msg_super!
        let this: id = msg_super![env; this init];

        if url != nil {
            retain(env, url);
        }
        env.objc.borrow_mut::<UIDocumentHostObject>(this).file_url = url;
        this
    }

    - (())dealloc {
        let url = env.objc.borrow::<UIDocumentHostObject>(this).file_url;
        if url != nil {
            release(env, url);
        }
        // Здесь тоже меняем на msg_super!
        msg_super![env; this dealloc]
    }

    - (id)fileURL {
        env.objc.borrow::<UIDocumentHostObject>(this).file_url
    }

    - (NSUInteger)documentState {
        0 // UIDocumentStateNormal
    }

    - (())openWithCompletionHandler:(id)completionHandler {
        call_bool_block(env, completionHandler, true);
    }

    - (())closeWithCompletionHandler:(id)completionHandler {
        call_bool_block(env, completionHandler, true);
    }

    - (())saveToURL:(id)_url forSaveOperation:(NSUInteger)_operation completionHandler:(id)completionHandler {
        call_bool_block(env, completionHandler, true);
    }

    - (())updateChangeCount:(NSUInteger)_change {
        // Ничего не делаем, просто принимаем вызов
    }

    - (id)localizedName {
        let url = env.objc.borrow::<UIDocumentHostObject>(this).file_url;
        if url != nil {
            msg![env; url lastPathComponent]
        } else {
            nil
        }
    }

    @end
};
