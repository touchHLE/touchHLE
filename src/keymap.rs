/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */
//! Keymapper: turns host keyboard input into synthetic touches.
//!
//! PearSilicon writes a per-app keymap JSON (see `PEAR_KEYMAP`) with bindings
//! whose points are normalized to the game viewport. 64-bit apps get these
//! translated by the injected runtime dylib; 32-bit apps run inside this
//! emulator, so the engine itself has to replay the same gestures against the
//! emulated touch screen.

use std::time::Duration;

/// Minimal JSON reader for keymaps.
///
/// The engine deliberately avoids a general-purpose JSON crate here: serde_json
/// adds blanket comparison impls that break type inference in unrelated
/// modules, and the keymap document is small and fully under our control.
mod json {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub enum Json {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<Json>),
        Obj(Vec<(String, Json)>),
    }

    impl Json {
        pub fn get(&self, name: &str) -> Option<&Json> {
            match self {
                Json::Obj(fields) => fields
                    .iter()
                    .find(|(field, _)| field == name)
                    .map(|(_, value)| value),
                _ => None,
            }
        }

        pub fn as_str(&self) -> Option<&str> {
            match self {
                Json::Str(value) => Some(value),
                _ => None,
            }
        }

        pub fn as_f64(&self) -> Option<f64> {
            match self {
                Json::Num(value) => Some(*value),
                _ => None,
            }
        }

        pub fn as_object(&self) -> Option<&[(String, Json)]> {
            match self {
                Json::Obj(fields) => Some(fields),
                _ => None,
            }
        }

        pub fn as_array(&self) -> Option<&[Json]> {
            match self {
                Json::Arr(values) => Some(values),
                _ => None,
            }
        }
    }

    pub fn parse(text: &str) -> Option<Json> {
        let bytes = text.as_bytes();
        let mut index = 0;
        let value = parse_value(bytes, &mut index)?;
        skip_whitespace(bytes, &mut index);
        (index == bytes.len()).then_some(value)
    }

    fn skip_whitespace(bytes: &[u8], index: &mut usize) {
        while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
            *index += 1;
        }
    }

    fn parse_value(bytes: &[u8], index: &mut usize) -> Option<Json> {
        skip_whitespace(bytes, index);
        match bytes.get(*index)? {
            b'{' => parse_object(bytes, index),
            b'[' => parse_array(bytes, index),
            b'"' => parse_string(bytes, index).map(Json::Str),
            b't' => literal(bytes, index, "true").then_some(Json::Bool(true)),
            b'f' => literal(bytes, index, "false").then_some(Json::Bool(false)),
            b'n' => literal(bytes, index, "null").then_some(Json::Null),
            _ => parse_number(bytes, index).map(Json::Num),
        }
    }

    fn literal(bytes: &[u8], index: &mut usize, word: &str) -> bool {
        if bytes.len() >= *index + word.len() && &bytes[*index..*index + word.len()] == word.as_bytes()
        {
            *index += word.len();
            true
        } else {
            false
        }
    }

    fn parse_object(bytes: &[u8], index: &mut usize) -> Option<Json> {
        *index += 1;
        let mut fields = Vec::new();
        loop {
            skip_whitespace(bytes, index);
            if bytes.get(*index) == Some(&b'}') {
                *index += 1;
                return Some(Json::Obj(fields));
            }
            let name = parse_string(bytes, index)?;
            skip_whitespace(bytes, index);
            if bytes.get(*index) != Some(&b':') {
                return None;
            }
            *index += 1;
            fields.push((name, parse_value(bytes, index)?));
            skip_whitespace(bytes, index);
            match bytes.get(*index) {
                Some(b',') => *index += 1,
                Some(b'}') => {
                    *index += 1;
                    return Some(Json::Obj(fields));
                }
                _ => return None,
            }
        }
    }

    fn parse_array(bytes: &[u8], index: &mut usize) -> Option<Json> {
        *index += 1;
        let mut values = Vec::new();
        loop {
            skip_whitespace(bytes, index);
            if bytes.get(*index) == Some(&b']') {
                *index += 1;
                return Some(Json::Arr(values));
            }
            values.push(parse_value(bytes, index)?);
            skip_whitespace(bytes, index);
            match bytes.get(*index) {
                Some(b',') => *index += 1,
                Some(b']') => {
                    *index += 1;
                    return Some(Json::Arr(values));
                }
                _ => return None,
            }
        }
    }

    fn parse_string(bytes: &[u8], index: &mut usize) -> Option<String> {
        if bytes.get(*index) != Some(&b'"') {
            return None;
        }
        *index += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = *bytes.get(*index)?;
            *index += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = *bytes.get(*index)?;
                    *index += 1;
                    match escape {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = bytes.get(*index..*index + 4)?;
                            *index += 4;
                            let code = u32::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(char::from_u32(code)?.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return None,
                    }
                }
                // Plain bytes may span multi-byte UTF-8 sequences; they are
                // validated once the string is complete.
                _ => out.push(byte),
            }
        }
    }

    fn parse_number(bytes: &[u8], index: &mut usize) -> Option<f64> {
        let start = *index;
        while *index < bytes.len()
            && matches!(bytes[*index], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        {
            *index += 1;
        }
        std::str::from_utf8(&bytes[start..*index])
            .ok()?
            .parse()
            .ok()
    }
}

/// A gesture step to be replayed `delay` after the key event that produced it.
pub type Scheduled = (Duration, TouchCmd);

/// Converts an SDL keycode to the canonical key name written by PearSilicon's
/// keymap editor. Returns `None` for keys the keymap cannot name.
pub fn key_name(keycode: sdl2::keyboard::Keycode) -> Option<String> {
    use sdl2::keyboard::Keycode as K;

    // Letters and digits share their own names.
    let raw = keycode.into_i32();
    if (b'a' as i32..=b'z' as i32).contains(&raw) {
        return Some((raw as u8 as char).to_string());
    }
    if (b'0' as i32..=b'9' as i32).contains(&raw) {
        return Some((raw as u8 as char).to_string());
    }
    if (K::F1.into_i32()..=K::F12.into_i32()).contains(&raw) {
        return Some(format!("f{}", raw - K::F1.into_i32() + 1));
    }

    Some(
        match keycode {
            K::Return | K::KpEnter => "enter",
            K::Escape => "escape",
            K::Backspace => "backspace",
            K::Delete => "delete",
            K::Tab => "tab",
            K::Space => "space",
            K::Left => "left",
            K::Right => "right",
            K::Up => "up",
            K::Down => "down",
            K::Home => "home",
            K::End => "end",
            K::PageUp => "pageup",
            K::PageDown => "pagedown",
            K::LShift | K::RShift => "shift",
            K::LCtrl | K::RCtrl => "control",
            K::LAlt | K::RAlt => "option",
            K::LGui | K::RGui => "command",
            K::CapsLock => "capslock",
            K::Minus => "minus",
            K::Equals => "equal",
            K::LeftBracket => "openbracket",
            K::RightBracket => "closebracket",
            K::Backslash => "backslash",
            K::Semicolon => "semicolon",
            K::Quote => "quote",
            K::Comma => "comma",
            K::Period => "period",
            K::Slash => "slash",
            K::Backquote => "grave",
            _ => return None,
        }
        .to_string(),
    )
}

/// A point in the game viewport, normalized to 0...1 with the origin at the
/// top-left corner.
pub type NormalizedPoint = (f32, f32);

/// A single gesture produced by a key press or release.
#[derive(Debug, Clone)]
pub enum TouchCmd {
    Down { id: u64, at: NormalizedPoint },
    Move { id: u64, at: NormalizedPoint },
    Up { id: u64, at: NormalizedPoint },
}

#[derive(Debug, Clone)]
enum Action {
    Tap(NormalizedPoint),
    Hold(NormalizedPoint),
    Swipe {
        from: NormalizedPoint,
        to: NormalizedPoint,
        duration: Duration,
    },
    Joystick {
        center: NormalizedPoint,
        radius: f32,
    },
    #[allow(dead_code)]
    MouseLook,
}

#[derive(Debug, Clone)]
struct Binding {
    key: String,
    action: Action,
}

/// Runtime state for one app's keymap.
pub struct KeymapState {
    bindings: Vec<Binding>,
    joystick: Option<(NormalizedPoint, f32)>,
    joy_up: bool,
    joy_down: bool,
    joy_left: bool,
    joy_right: bool,
    joy_active: bool,
    joy_touch: NormalizedPoint,
    next_id: u64,
    /// Where the keymap was loaded from, when loaded via [`Self::from_env`].
    /// Enables hot-reload via [`Self::reload_if_changed`].
    source_path: Option<std::path::PathBuf>,
    last_modified: Option<std::time::SystemTime>,
    reload_countdown: u8,
}

impl KeymapState {
    /// Loads the keymap named by `PEAR_KEYMAP`, if the host set one.
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("PEAR_KEYMAP")?;
        if path.is_empty() {
            return None;
        }
        let json = std::fs::read_to_string(&path).ok()?;
        let mut state = Self::from_json(&json);
        if let Some(state) = &mut state {
            let path = std::path::PathBuf::from(path);
            log!("Keymapper: loaded {}", path.display());
            state.last_modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            state.source_path = Some(path);
        }
        state
    }

    /// Re-reads the source keymap file if it changed on disk, applying edits
    /// without an app restart. Called from the event loop; the check is
    /// throttled to a couple of times per second.
    pub fn reload_if_changed(&mut self) {
        let Some(path) = self.source_path.clone() else {
            return;
        };
        self.reload_countdown = self.reload_countdown.saturating_sub(1);
        if self.reload_countdown > 0 {
            return;
        }
        self.reload_countdown = 60;

        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if modified == self.last_modified {
            return;
        }
        self.last_modified = modified;

        match std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| Self::from_json(&json))
        {
            Some(new) => {
                self.bindings = new.bindings;
                self.joystick = new.joystick;
                // Reset transient state so stale touches don't leak through.
                self.joy_up = false;
                self.joy_down = false;
                self.joy_left = false;
                self.joy_right = false;
                self.joy_active = false;
                log!("Keymapper: reloaded {}", path.display());
            }
            None => {
                log!("Keymapper: reload failed, keeping previous bindings");
            }
        }
    }

    /// Loads a keymap from JSON. Returns `None` when the document has no
    /// keyboard bindings, so callers can skip installing the keymapper.
    pub fn from_json(json: &str) -> Option<Self> {
        let root = json::parse(json)?;
        let mut bindings = Vec::new();
        let mut joystick = None;

        for raw in root.get("bindings")?.as_array()? {
            // Controller bindings are handled by the host's GameController
            // bridge, not by keyboard translation.
            if raw.get("source").and_then(json::Json::as_str).unwrap_or("keyboard") != "keyboard" {
                continue;
            }
            let Some(key) = raw.get("key").and_then(json::Json::as_str) else {
                continue;
            };
            let key = key.to_ascii_lowercase();
            let Some(action) = raw.get("action").and_then(Self::parse_action) else {
                continue;
            };
            if let Action::Joystick { center, radius } = action {
                joystick = Some((center, radius));
            }
            bindings.push(Binding { key, action });
        }

        if bindings.is_empty() {
            return None;
        }
        Some(Self {
            bindings,
            joystick,
            joy_up: false,
            joy_down: false,
            joy_left: false,
            joy_right: false,
            joy_active: false,
            joy_touch: (0.0, 0.0),
            next_id: 1,
            source_path: None,
            last_modified: None,
            reload_countdown: 0,
        })
    }

    fn parse_action(action: &json::Json) -> Option<Action> {
        let object = action.as_object()?;
        let (kind, body) = object.iter().next()?;
        match kind.as_str() {
            // Unlabeled associated values are bridged as "_0" by Swift's
            // synthesized Codable encoding.
            "tap" => Some(Action::Tap(Self::point(body.get("_0")?)?)),
            "hold" => Some(Action::Hold(Self::point(body.get("_0")?)?)),
            "joystick" => Some(Action::Joystick {
                center: Self::point(body.get("center")?)?,
                radius: body.get("radius").and_then(json::Json::as_f64).unwrap_or(0.1) as f32,
            }),
            "swipe" => Some(Action::Swipe {
                from: Self::point(body.get("from")?)?,
                to: Self::point(body.get("to")?)?,
                duration: Duration::from_secs_f32(
                    body.get("duration")
                        .and_then(json::Json::as_f64)
                        .unwrap_or(0.25) as f32,
                ),
            }),
            "mouseLook" => Some(Action::MouseLook),
            _ => None,
        }
    }

    fn point(value: &json::Json) -> Option<NormalizedPoint> {
        Some((
            value.get("x")?.as_f64()? as f32,
            value.get("y")?.as_f64()? as f32,
        ))
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }

    /// Handles a keyboard event. `key` uses the canonical names written by the
    /// keymap editor (`space`, `shift`, `f`, `1`, `tab`, …).
    /// Returns `None` when the key is not bound, so the caller can keep its
    /// normal keyboard handling.
    pub fn key_event(&mut self, key: &str, pressed: bool) -> Option<Vec<Scheduled>> {
        let mut commands = Vec::new();

        // W/A/S/D drive the virtual joystick when one exists, exactly like the
        // 64-bit runtime does.
        if self.joystick.is_some() {
            let handled = match key {
                "w" => Some(&mut self.joy_up),
                "s" => Some(&mut self.joy_down),
                "a" => Some(&mut self.joy_left),
                "d" => Some(&mut self.joy_right),
                _ => None,
            };
            if let Some(flag) = handled {
                *flag = pressed;
                return Some(self.update_joystick());
            }
        }

        let action = self
            .bindings
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.action.clone());
        let Some(action) = action else {
            return None;
        };

        match action {
            Action::Tap(at) => {
                if pressed {
                    // A tap is a complete gesture: down on press, up shortly
                    // after, so the app sees a real finger dwell.
                    let id = self.take_id();
                    commands.push((Duration::ZERO, TouchCmd::Down { id, at }));
                    commands.push((Duration::from_millis(60), TouchCmd::Up { id, at }));
                }
            }
            Action::Hold(at) => {
                // Hold keeps a stable finger id for the lifetime of the press so
                // repeated keys don't produce orphaned fingers.
                let id = self.binding_id(key);
                commands.push((
                    Duration::ZERO,
                    if pressed {
                        TouchCmd::Down { id, at }
                    } else {
                        TouchCmd::Up { id, at }
                    },
                ));
            }
            Action::Swipe {
                from,
                to,
                duration,
            } => {
                if pressed {
                    // Spread the drag over the gesture duration so the app can
                    // sample intermediate positions.
                    let id = self.take_id();
                    let step = Duration::from_millis(16);
                    let steps = (duration.as_millis() / 16).max(1) as u32;
                    commands.push((Duration::ZERO, TouchCmd::Down { id, at: from }));
                    for index in 1..steps {
                        let t = index as f32 / steps as f32;
                        commands.push((
                            step * index,
                            TouchCmd::Move {
                                id,
                                at: (
                                    from.0 + (to.0 - from.0) * t,
                                    from.1 + (to.1 - from.1) * t,
                                ),
                            },
                        ));
                    }
                    commands.push((duration, TouchCmd::Move { id, at: to }));
                    commands.push((duration, TouchCmd::Up { id, at: to }));
                }
            }
            Action::Joystick { .. } | Action::MouseLook => {}
        }
        Some(commands)
    }

    /// Stable finger id for a key that holds a finger down.
    fn binding_id(&self, key: &str) -> u64 {
        // Offset by a large base so ids never collide with the transient ids
        // handed out by `take_id`.
        const HOLD_BASE: u64 = 0x4000_0000;
        HOLD_BASE + key.bytes().fold(0u64, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte as u64)) % 0x1000
    }

    fn update_joystick(&mut self) -> Vec<Scheduled> {
        let Some((center, radius)) = self.joystick else {
            return Vec::new();
        };
        let dx = (self.joy_right as i32 - self.joy_left as i32) as f32;
        let dy = (self.joy_down as i32 - self.joy_up as i32) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let want = length > 0.0;

        let id = u64::MAX - 1;
        let target = if length > 0.0 {
            (
                center.0 + dx / length * radius,
                center.1 + dy / length * radius,
            )
        } else {
            center
        };

        if want && !self.joy_active {
            self.joy_active = true;
            self.joy_touch = center;
            // Touch down at the stick centre, then drag toward the direction,
            // matching how mobile on-screen sticks derive a direction.
            vec![
                (Duration::ZERO, TouchCmd::Down { id, at: center }),
                (Duration::ZERO, TouchCmd::Move { id, at: target }),
            ]
        } else if !want && self.joy_active {
            self.joy_active = false;
            vec![(
                Duration::ZERO,
                TouchCmd::Up {
                    id,
                    at: self.joy_touch,
                },
            )]
        } else if want {
            self.joy_touch = target;
            vec![(Duration::ZERO, TouchCmd::Move { id, at: target })]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "bindings": [
            {"key": "space", "source": "keyboard", "action": {"tap": {"_0": {"x": 0.5, "y": 0.9}}}},
            {"key": "shift", "source": "keyboard", "action": {"hold": {"_0": {"x": 0.9, "y": 0.9}}}},
            {"key": "wasd", "source": "keyboard", "action": {"joystick": {"center": {"x": 0.1, "y": 0.7}, "radius": 0.1}}},
            {"key": "buttonA", "source": "controller", "action": {"tap": {"_0": {"x": 0.1, "y": 0.1}}}}
        ]
    }"#;

    #[test]
    fn tap_emits_a_complete_down_up_pair() {
        let mut map = KeymapState::from_json(SAMPLE).unwrap();
        let commands = map.key_event("space", true).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].0, Duration::ZERO);
        assert!(matches!(commands[0].1, TouchCmd::Down { at, .. } if at == (0.5, 0.9)));
        assert!(commands[1].0 > Duration::ZERO);
        assert!(matches!(commands[1].1, TouchCmd::Up { at, .. } if at == (0.5, 0.9)));
    }

    #[test]
    fn hold_tracks_press_and_release_with_one_finger() {
        let mut map = KeymapState::from_json(SAMPLE).unwrap();
        let down = map.key_event("shift", true).unwrap();
        let up = map.key_event("shift", false).unwrap();
        let (TouchCmd::Down { id: down_id, .. }) = down[0].1 else {
            panic!("expected touch down");
        };
        let TouchCmd::Up { id: up_id, .. } = up[0].1 else {
            panic!("expected touch up");
        };
        assert_eq!(down_id, up_id);
    }

    #[test]
    fn joystick_starts_at_centre_and_drags_with_wasd() {
        let mut map = KeymapState::from_json(SAMPLE).unwrap();
        let started = map.key_event("d", true).unwrap();
        assert!(matches!(started[0].1, TouchCmd::Down { at, .. } if at == (0.1, 0.7)));
        assert!(matches!(started[1].1, TouchCmd::Move { at, .. } if at.0 > 0.1));
        let stopped = map.key_event("d", false).unwrap();
        assert!(matches!(stopped[0].1, TouchCmd::Up { .. }));
    }

    #[test]
    fn swipe_is_spread_over_its_duration() {
        let json = r#"{"bindings": [{"key": "f", "source": "keyboard",
            "action": {"swipe": {"from": {"x": 0.3, "y": 0.5}, "to": {"x": 0.7, "y": 0.5}, "duration": 0.32}}}]}"#;
        let mut map = KeymapState::from_json(json).unwrap();
        let commands = map.key_event("f", true).unwrap();
        assert!(matches!(commands[0].1, TouchCmd::Down { at, .. } if at == (0.3, 0.5)));
        assert!(matches!(commands[commands.len() - 1].1, TouchCmd::Up { at, .. } if at == (0.7, 0.5)));
        assert!(commands.iter().any(|(delay, _)| *delay > Duration::from_millis(100)));
        assert!(map.key_event("f", false).is_some_and(|c| c.is_empty()));
    }

    #[test]
    fn unbound_keys_are_reported_as_unhandled() {
        let mut map = KeymapState::from_json(SAMPLE).unwrap();
        assert!(map.key_event("q", true).is_none());
        assert!(map.key_event("buttona", true).is_none());
    }

    #[test]
    fn missing_bindings_disable_the_keymapper() {
        assert!(KeymapState::from_json("{}").is_none());
        assert!(KeymapState::from_json(r#"{"bindings": []}"#).is_none());
    }

    #[test]
    fn key_names_use_the_editor_vocabulary() {
        use sdl2::keyboard::Keycode as K;
        assert_eq!(key_name(K::A).as_deref(), Some("a"));
        assert_eq!(key_name(K::Num1).as_deref(), Some("1"));
        assert_eq!(key_name(K::Return).as_deref(), Some("enter"));
        assert_eq!(key_name(K::LShift).as_deref(), Some("shift"));
        assert_eq!(key_name(K::F5).as_deref(), Some("f5"));
        assert_eq!(key_name(K::PageUp).as_deref(), Some("pageup"));
    }
}
