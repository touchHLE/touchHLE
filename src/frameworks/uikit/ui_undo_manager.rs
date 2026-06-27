/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! NSUndoManager implementation

use crate::objc::{id, msg, msg_class, nil, AutoreleasePoolPtr, ClassExports, HostObject, NSZonePtr, SEL};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::MutPtr;
use crate::{Environment,framework_state};

#[derive(Default)]
pub struct NSUndoManagerState {
    pub undo_stack: Vec<id>,
    pub redo_stack: Vec<id>,
    pub is_undoing: bool,
    pub is_redoing: bool,
}

struct NSUndoManagerHostObject {
    delegate: id,
    levels: usize,
    groups_by_age: usize,
}
impl HostObject for NSUndoManagerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSUndoManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSUndoManagerHostObject {
        delegate: nil,
        levels: 1,
        groups_by_age: 1,
    });
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (id)retain { this }
- (id)autorelease { this }
- (())release {}

- (bool)isUndoing {
    framework_state!(env).uikit.undo_manager.is_undoing
}

- (bool)isRedoing {
    framework_state!(env).uikit.undo_manager.is_redoing
}

- (bool)canUndo {
    !framework_state!(env).uikit.undo_manager.undo_stack.is_empty()
}

- (bool)canRedo {
    !framework_state!(env).uikit.undo_manager.redo_stack.is_empty()
}

- (())undo {
    let state = &mut framework_state!(env).uikit.undo_manager;
    if let Some(invocation) = state.undo_stack.pop() {
        state.is_undoing = true;
        let _: () = msg![env; invocation invoke];
        state.is_undoing = false;
        state.redo_stack.push(invocation);
    }
}

- (())redo {
    let state = &mut framework_state!(env).uikit.undo_manager;
    if let Some(invocation) = state.redo_stack.pop() {
        state.is_redoing = true;
        let _: () = msg![env; invocation invoke];
        state.is_redoing = false;
        state.undo_stack.push(invocation);
    }
}

- (())removeAllActions {
    let state = &mut framework_state!(env).uikit.undo_manager;
    state.undo_stack.clear();
    state.redo_stack.clear();
}

- (())removeAllActionsWithTarget:(id)target {
    let state = &mut framework_state!(env).uikit.undo_manager;
    state.undo_stack.retain(|inv| {
        let inv_target: id = msg![env; inv target];
        inv_target != target
    });
    state.redo_stack.retain(|inv| {
        let inv_target: id = msg![env; inv target];
        inv_target != target
    });
}

- (NSUInteger)undoCount {
    framework_state!(env).uikit.undo_manager.undo_stack.len() as NSUInteger
}

- (NSUInteger)redoCount {
    framework_state!(env).uikit.undo_manager.redo_stack.len() as NSUInteger
}

- (())registerUndoWithTarget:(id)target
                    selector:(SEL)selector
                      object:(id)object {
    let signature: id = msg_class![env; NSMethodSignature signatureWithObjCTypes:"v@:@"];

    let invocation: id = msg_class![env; NSInvocation invocationWithMethodSignature:signature];
    msg![env; invocation setTarget:target];
    msg![env; invocation setSelector:selector];
    if object != nil {
        let _: () = msg![env; invocation setArgument:object atIndex:3];
    }

    let state = &mut framework_state!(env).uikit.undo_manager;
    state.undo_stack.push(invocation);
    // Clear redo stack when new action is registered
    state.redo_stack.clear();
}

- (())forwardInvocation:(id)invocation {
    let target: id = msg![env; invocation target];
    if target != nil {
        let _: () = msg![env; invocation invoke];
    }
}

- (id)methodSignatureForSelector:(SEL)selector {
    // Return a dummy method signature to prevent crashes
    msg_class![env; NSMethodSignature signatureWithObjCTypes:"v@:"]
}

- (id)delegate {
    env.objc.borrow::<NSUndoManagerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.delegate = delegate;
}

- (NSInteger)levels {
    env.objc.borrow::<NSUndoManagerHostObject>(this).levels as NSInteger
}

- (())setLevels:(NSInteger)levels {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.levels = levels as usize;
}

- (())beginUndoGrouping {
    // Stub - grouping handled implicitly
}

- (())endUndoGrouping {
    // Stub - grouping handled implicitly
}

- (NSInteger)groupingLevel {
    0
}

- (())endAllGrouping {
    // Stub
}

- (())setGroupsByEvent:(bool)groupsByEvent {
    let host_object = env.objc.borrow_mut::<NSUndoManagerHostObject>(this);
    host_object.groups_by_age = if groupsByEvent { 1 } else { 0 };
}

- (bool)groupsByEvent {
    let host_object = env.objc.borrow::<NSUndoManagerHostObject>(this);
    host_object.groups_by_age != 0
}

- (())undoNestedGroup {
    msg![env; this undo]
}

- (id)undoActionName {
    if msg![env; this canUndo] {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        let name = crate::frameworks::foundation::ns_string::from_rust_string(env, "Undo".to_string());
        let result = crate::objc::retain(env, name);
        let _: () = msg![env; pool drain];
        result
    } else {
        nil
    }
}

- (id)redoActionName {
    if msg![env; this canRedo] {
        let pool: id = msg_class![env; NSAutoreleasePool new];
        let name = crate::frameworks::foundation::ns_string::from_rust_string(env, "Redo".to_string());
        let result = crate::objc::retain(env, name);
        let _: () = msg![env; pool drain];
        result
    } else {
        nil
    }
}

- (id)undoMenuItemTitle {
    msg![env; this undoActionName]
}

- (id)redoMenuItemTitle {
    msg![env; this redoMenuItemTitle]
}

- (())setUndoManager:(id)manager {
    // Stub - not needed for basic implementation
}

@end

};
