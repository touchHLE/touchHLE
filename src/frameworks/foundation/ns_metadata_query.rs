/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
pub struct NSMetadataQueryHostObject {
    is_started: bool,
}

// Заменяем неработающий макрос на прямую реализацию трейта HostObject
impl HostObject for NSMetadataQueryHostObject {}


pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSMetadataQuery: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::<NSMetadataQueryHostObject>::default();
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    // Метод - (id)init удален, так как базовый NSObject уже
    // предоставляет стандартную инициализацию, и нам не нужно
    // конфликтовать с ключевым словом super.

    - (())setSearchScopes:(id)_scopes {}

    - (())setPredicate:(id)_predicate {}

    - (())setSortDescriptors:(id)_descriptors {}

    - (bool)startQuery {
        env.objc.borrow_mut::<NSMetadataQueryHostObject>(this).is_started = true;

        // Полноценная логика: после "поиска" файлов мы должны уведомить
        // систему,
        // что сбор данных завершен.
        // Так игра поймет, что можно проверять результаты.
        let notification_center: id = msg_class![env; NSNotificationCenter defaultCenter];
        let name = get_static_str(env, "NSMetadataQueryDidFinishGatheringNotification");
        () = msg![env; notification_center postNotificationName:name object:this];

        true
    }

    - (())stopQuery {
        env.objc.borrow_mut::<NSMetadataQueryHostObject>(this).is_started = false;
    }

    - (())disableUpdates {}

    - (())enableUpdates {}

    - (id)results {
        // Возвращаем пустой NSArray, так как облачных файлов нет.
        msg_class![env; NSArray array]
    }

    - (u32)resultCount {
        0
    }

    @end
};
