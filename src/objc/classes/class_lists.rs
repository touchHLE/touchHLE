/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Separate module just for the class lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{
    av_audio, core_animation, core_foundation, core_graphics, core_location, foundation, game_kit,
    media_player, opengles, store_kit, uikit,
};

/// All the lists of classes that the runtime should search through.
pub const CLASS_LISTS: &[super::ClassExports] = &[
    crate::app_picker::CLASSES, // Not a framework! Special internal classes.
    core_animation::ca_animation::CLASSES,
    core_animation::ca_eagl_layer::CLASSES,
    core_animation::ca_layer::CLASSES,
    core_animation::ca_media_timing_function::CLASSES,
    core_graphics::cg_data_provider::CLASSES,
    core_graphics::cg_color::CLASSES,
    core_graphics::cg_color_space::CLASSES,
    core_graphics::cg_context::CLASSES,
    core_graphics::cg_image::CLASSES,
    core_foundation::cf_run_loop_timer::CLASSES, // Special internal classes.
    core_location::CLASSES,
    game_kit::gk_local_player::CLASSES,
    game_kit::gk_score::CLASSES,
    foundation::ns_array::CLASSES,
    foundation::ns_autorelease_pool::CLASSES,
    foundation::ns_bundle::CLASSES,
    foundation::ns_character_set::CLASSES,
    foundation::ns_coder::CLASSES,
    foundation::ns_data::CLASSES,
    foundation::ns_date::CLASSES,
    foundation::ns_date_formatter::CLASSES,
    foundation::ns_dictionary::CLASSES,
    foundation::ns_enumerator::CLASSES,
    foundation::ns_error::CLASSES,
    foundation::ns_file_handle::CLASSES,
    foundation::ns_file_manager::CLASSES,
    foundation::ns_keyed_unarchiver::CLASSES,
    foundation::ns_locale::CLASSES,
    foundation::ns_lock::CLASSES,
    foundation::ns_notification::CLASSES,
    foundation::ns_notification_center::CLASSES,
    foundation::ns_null::CLASSES,
    foundation::ns_object::CLASSES,
    foundation::ns_process_info::CLASSES,
    foundation::ns_property_list_serialization::CLASSES,
    foundation::ns_run_loop::CLASSES,
    foundation::ns_set::CLASSES,
    foundation::ns_string::CLASSES,
    foundation::ns_thread::CLASSES,
    foundation::ns_timer::CLASSES,
    foundation::ns_time_zone::CLASSES,
    foundation::ns_url::CLASSES,
    foundation::ns_url_connection::CLASSES,
    foundation::ns_url_request::CLASSES,
    foundation::ns_user_defaults::CLASSES,
    foundation::ns_value::CLASSES,
    foundation::ns_xml_parser::CLASSES,
    av_audio::av_audio_player::CLASSES,
    media_player::movie_player::CLASSES,
    media_player::music_player::CLASSES,
    media_player::media_library::CLASSES,
    media_player::media_picker_controller::CLASSES,
    media_player::media_query::CLASSES,
    opengles::eagl::CLASSES,
    store_kit::sk_payment_queue::CLASSES,
    store_kit::sk_product::CLASSES,
    uikit::ui_accelerometer::CLASSES,
    uikit::ui_activity_indicator_view::CLASSES,
    uikit::ui_application::CLASSES,
    uikit::ui_color::CLASSES,
    uikit::ui_device::CLASSES,
    uikit::ui_event::CLASSES,
    uikit::ui_font::CLASSES,
    uikit::ui_image::CLASSES,
    uikit::ui_image_picker_controller::CLASSES,
    uikit::ui_nib::CLASSES,
    uikit::ui_responder::CLASSES,
    uikit::ui_screen::CLASSES,
    uikit::ui_touch::CLASSES,
    uikit::ui_view::CLASSES,
    uikit::ui_view::ui_alert_view::CLASSES,
    uikit::ui_view::ui_control::CLASSES,
    uikit::ui_view::ui_control::ui_button::CLASSES,
    uikit::ui_view::ui_control::ui_segmented_control::CLASSES,
    uikit::ui_view::ui_control::ui_slider::CLASSES,
    uikit::ui_view::ui_control::ui_text_field::CLASSES,
    uikit::ui_view::ui_control::ui_switch::CLASSES,
    uikit::ui_view::ui_image_view::CLASSES,
    uikit::ui_view::ui_label::CLASSES,
    uikit::ui_view::ui_picker_view::CLASSES,
    uikit::ui_view::ui_scroll_view::CLASSES,
    uikit::ui_view::ui_scroll_view::ui_text_view::CLASSES,
    uikit::ui_view::ui_web_view::CLASSES,
    uikit::ui_view::ui_window::CLASSES,
    uikit::ui_view_controller::CLASSES,
    uikit::ui_view_controller::ui_navigation_controller::CLASSES,
];

#[cfg(test)]
mod tests {
    use crate::objc::ClassTemplate;

    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_classes() {
        let mut seen_classes = HashSet::new();

        for &class_list in CLASS_LISTS {
            for (class_name, template) in class_list {
                if !seen_classes.insert(class_name) {
                    panic!("Found duplicate class export {}", class_name);
                }
                let ClassTemplate {
                    class_methods,
                    instance_methods,
                    ..
                } = template;

                let mut seen_class_methods = HashSet::with_capacity(class_methods.len());

                for (method_name, _) in *class_methods {
                    if !seen_class_methods.insert(method_name) {
                        panic!(
                            "Found duplicate class method {} for class {}",
                            method_name, class_name
                        )
                    }
                }

                let mut seen_instance_methods = HashSet::with_capacity(instance_methods.len());

                for (method_name, _) in *instance_methods {
                    if !seen_instance_methods.insert(method_name) {
                        panic!(
                            "Found duplicate instance method {} for class {}",
                            method_name, class_name
                        )
                    }
                }
            }
        }
    }
}
