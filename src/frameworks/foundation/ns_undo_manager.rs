use crate::objc::{id, SEL};
use crate::objc_classes;
use std::collections::HashMap;

pub struct UndoAction {
    pub target: id,
    pub selector: SEL,
    pub object: id,
}

#[derive(Default)]
pub struct State {
    pub stacks: HashMap<id, Vec<UndoAction>>,
}

pub const CLASSES: crate::objc::ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSUndoManager : NSObject

    - (id) init {
        env.framework_state.foundation.ns_undo_manager.stacks.insert(this, Vec::new());
        this
    }

    - (()) dealloc {
        env.framework_state.foundation.ns_undo_manager.stacks.remove(&this);
    }

    - (()) registerUndoWithTarget:(id)target selector:(SEL)selector object:(id)anObject {
        if let Some(stack) = env.framework_state.foundation.ns_undo_manager.stacks.get_mut(&this) {
            stack.push(UndoAction { target, selector, object: anObject });
            // println!("NSUndoManager: Зарегистрировано действие. Всего в
            // стеке: {}", stack.len());
        }
    }

    - (()) removeAllActions {
        if let Some(stack) = env.framework_state.foundation.ns_undo_manager.stacks.get_mut(&this) {
            stack.clear();
        }
    }

    - (bool) canUndo {
        env.framework_state.foundation.ns_undo_manager.stacks.get(&this).is_some_and(|s| !s.is_empty())
    }

    - (()) undo {
        if let Some(stack) = env.framework_state.foundation.ns_undo_manager.stacks.get_mut(&this) {
            if let Some(_action) = stack.pop() {
                // ВАЖНО: Прямой вызов гостевого objc_msgSend из хоста требует
                // настройки CPU.
                // Пока мы просто извлекаем действие, чтобы эмулятор не
                // паниковал,
                // и игра могла продолжить свою работу без вылета.
                println!("NSUndoManager: Вызван метод undo! (Гостевой вызов пропущен)");
            }
        }
    }

    @end
};
