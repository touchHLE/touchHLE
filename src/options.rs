/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Parsing and management of user-configurable options, e.g. for input methods.

use crate::window::DeviceOrientation;
use std::num::NonZeroU32;

pub const DOCUMENTATION: &str = "\
View options:
    --landscape-left
    --landscape-right
        Changes the orientation the virtual device will have at startup.
        The default is portrait.

        --landscape-left means rotate 90° counterclockwise from portrait.
        --landscape-right means rotate 90° clockwise from portrait.

        Usually apps that require landscape mode will tell touchHLE about this,
        and it will automatically rotate the window, but some apps neglect to
        do this. These options may be useful in that case.

    --scale-hack=...
        Set a scaling factor for the window. touchHLE will attempt to run the
        app with an increased internal resolution. This is a hack and there's
        no guarantee it will work correctly for all apps.

        The default is no scale hack, which is equivalent to a value of 1 (i.e.
        a scale of 1×).

        This is a natural number that is at least 1.

Game controller options:
    --deadzone=...
        Configures the size of the \"dead zone\" for analog stick inputs.

        The default value is 0.1, which means that 10% of the stick's range on
        the X and Y axes around the center position will be collapsed into a
        single point, so that movements in that range are ignored.

        This is a floating-point (decimal) number between 0 and 1.

    --x-tilt-range=...
    --y-tilt-range=...
        Set the simulated rotation range of the device on its X or Y axis.

        By default, an analog stick's axis is mapped to a rotation range of 60°
        (30° in either direction). If you wanted a range of 90° on the X axis,
        you could use --x-tilt-range=90.

        Note that the device's X axis is mapped to the analog stick's Y axis
        and vice-versa, because tilting the device to the left means rotating
        it on its Y axis, and so on.

        This is a floating-point (decimal) number of degrees, without a degree
        symbol. It may be negative.

    --x-tilt-offset=...
    --y-tilt-offset=...
        Offset the simulated angle of the device on its X or Y axis.

        By default, the device is simulated as being level with the ground when
        the stick is in the center/neutral position. This option is intended for
        games that use a different angle relative to the ground as their neutral
        position. For example, if a game expects you to hold the device in a
        landscape orientation, with a 45° angle to the ground, you might use
        --y-tilt-offset=45.

        Note that the device's X axis is mapped to the analog stick's Y axis
        and vice-versa, because tilting the device to the left means rotating
        it on its Y axis, and so on.

        This is a floating-point (decimal) number of degrees, without a degree
        symbol. It may be negative.

Debugging options:
    --breakpoint=...
        This option sets a primitive breakpoint at a provided memory address.
        The target instruction will be overwritten shortly after the binary is
        loaded, and executing the instruction will cause touchHLE to panic.

        The address is hexadecimal and can have an optional '0x' prefix.
        If the target instruction is a Thumb instruction, either the lowest bit
        of the address must be set, or the address should be prefixed with 'T',
        e.g. 'T0xF00' or 'TF00'.

        To set multiple breakpoints, use several '--breakpoint=' arguments.

    --disable-direct-memory-access
        Force dynarmic to always access guest memory via the memory access
        callbacks, rather than using the fast direct access path (page tables).
";

/// Struct containing all user-configurable options.
pub struct Options {
    pub initial_orientation: DeviceOrientation,
    pub scale_hack: NonZeroU32,
    pub deadzone: f32,
    pub x_tilt_range: f32,
    pub y_tilt_range: f32,
    pub x_tilt_offset: f32,
    pub y_tilt_offset: f32,
    pub breakpoints: Vec<u32>,
    pub direct_memory_access: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            initial_orientation: DeviceOrientation::Portrait,
            scale_hack: NonZeroU32::new(1).unwrap(),
            deadzone: 0.1,
            x_tilt_range: 60.0,
            y_tilt_range: 60.0,
            x_tilt_offset: 0.0,
            y_tilt_offset: 0.0,
            breakpoints: Vec::new(),
            direct_memory_access: true,
        }
    }
}

impl Options {
    /// Parse the command-line argument syntax for an option. Returns `Ok(true)`
    /// if the option was valid and has been applied, and `Ok(false)` if the
    /// option was not recognized.
    pub fn parse_argument(&mut self, arg: &str) -> Result<bool, String> {
        fn parse_degrees(arg: &str, name: &str) -> Result<f32, String> {
            let arg: f32 = arg
                .parse()
                .map_err(|_| format!("Value for {} is invalid", name))?;
            if !arg.is_finite() || !(-360.0..=360.0).contains(&arg) {
                return Err(format!("Value for {} is out of range", name));
            }
            Ok(arg)
        }

        if arg == "--landscape-left" {
            self.initial_orientation = DeviceOrientation::LandscapeLeft;
        } else if arg == "--landscape-right" {
            self.initial_orientation = DeviceOrientation::LandscapeRight;
        } else if let Some(value) = arg.strip_prefix("--scale-hack=") {
            self.scale_hack = value
                .parse()
                .map_err(|_| "Invalid scale hack factor".to_string())?;
        } else if let Some(value) = arg.strip_prefix("--deadzone=") {
            self.deadzone = parse_degrees(value, "deadzone")?;
        } else if let Some(value) = arg.strip_prefix("--x-tilt-range=") {
            self.x_tilt_range = parse_degrees(value, "X tilt range")?;
        } else if let Some(value) = arg.strip_prefix("--y-tilt-range=") {
            self.y_tilt_range = parse_degrees(value, "Y tilt range")?;
        } else if let Some(value) = arg.strip_prefix("--x-tilt-offset=") {
            self.x_tilt_offset = parse_degrees(value, "X tilt offset")?;
        } else if let Some(value) = arg.strip_prefix("--y-tilt-offset=") {
            self.y_tilt_offset = parse_degrees(value, "Y tilt offset")?;
        } else if let Some(addr) = arg.strip_prefix("--breakpoint=") {
            let is_thumb = addr.starts_with('T');
            let addr = addr.strip_prefix('T').unwrap_or(addr);
            let addr = addr.strip_prefix("0x").unwrap_or(addr);
            let addr = u32::from_str_radix(addr, 16)
                .map_err(|_| "Incorrect breakpoint syntax".to_string())?;
            self.breakpoints
                .push(if is_thumb { addr | 0x1 } else { addr });
        } else if arg == "--disable-direct-memory-access" {
            self.direct_memory_access = false;
        } else {
            return Ok(false);
        };
        Ok(true)
    }
}
