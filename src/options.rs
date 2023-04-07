/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Parsing and management of user-configurable options, e.g. for input methods.

use crate::window::DeviceOrientation;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, ToSocketAddrs};
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
    --disable-direct-memory-access
        Force dynarmic to always access guest memory via the memory access
        callbacks, rather than using the fast direct access path (page tables).

    --gdb=...
        Starts touchHLE in debugging mode, listening for GDB remote serial
        protocol connections over TCP on the specified host and port.

        You can then connect to touchHLE with GDB and make use of its features
        to inspect memory and registers, set up software breakpoints, and
        continue or step execution.

        The host and port should be separated by a colon. The host can be a
        host name or an IP address. IPv6 addresses should be enclosed in square
        brackets, e.g. --gdb=[::1]:9001 for IPv6 loopback device port 9001.
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
    pub direct_memory_access: bool,
    pub gdb_listen_addrs: Option<Vec<SocketAddr>>,
}

impl Default for Options {
    #[cfg(not(target_os = "android"))]
    fn default() -> Self {
        Options {
            initial_orientation: DeviceOrientation::Portrait,
            scale_hack: NonZeroU32::new(1).unwrap(),
            deadzone: 0.1,
            x_tilt_range: 60.0,
            y_tilt_range: 60.0,
            x_tilt_offset: 0.0,
            y_tilt_offset: 0.0,
            direct_memory_access: true,
            gdb_listen_addrs: None,
        }
    }
    #[cfg(target_os = "android")]
    // Those are a bit arbitrary to "look good" for SMB on Pixel 3a
    // TODO: properly calculate scale, position and orientation for mobile devices
    fn default() -> Self {
        Options {
            initial_orientation: DeviceOrientation::LandscapeLeft,
            scale_hack: NonZeroU32::new(3).unwrap(),
            deadzone: 0.1,
            x_tilt_range: 60.0,
            y_tilt_range: 60.0,
            x_tilt_offset: 0.0,
            y_tilt_offset: 24.0,
            direct_memory_access: true,
            gdb_listen_addrs: None,
        }
    }
}

impl Options {
    /// Parse the command-line argument syntax for an option. Returns `Ok(true)`
    /// if the option was valid and has been applied, or `Ok(false)` if the
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
        } else if arg == "--disable-direct-memory-access" {
            self.direct_memory_access = false;
        } else if let Some(address) = arg.strip_prefix("--gdb=") {
            let addrs = address
                .to_socket_addrs()
                .map_err(|e| format!("Could not resolve GDB server listen address: {}", e))?
                .collect();
            println!("{:?}", addrs);
            self.gdb_listen_addrs = Some(addrs);
        } else {
            return Ok(false);
        };
        Ok(true)
    }
}

/// Name of the file containing touchHLE's default options for various apps.
pub const DEFAULTS_FILENAME: &str = "touchHLE_default_options.txt";
/// Name of the file intended for the user's own options.
pub const USER_FILENAME: &str = "touchHLE_options.txt";

/// Try to get app-specific options from a file.
///
/// Returns [Ok] if there is no error when reading the file, otherwise [Err].
/// The [Ok] value is a [Some] with the options if they could be found, or
/// [None] if no options were found for this app.
pub fn get_options_from_file(filename: &str, app_id: &str) -> Result<Option<String>, String> {
    let file = File::open(filename).map_err(|e| format!("Could not open {}: {}", filename, e))?;

    let file = BufReader::new(file);
    for (line_no, line) in BufRead::lines(file).enumerate() {
        // Line numbering usually starts from 1
        let line_no = line_no + 1;

        let line = line.map_err(|e| {
            format!(
                "Error while reading line {} of {}: {}",
                line_no, filename, e
            )
        })?;

        // # for single-line comments
        let line = if let Some((rest, _)) = line.split_once('#') {
            rest
        } else {
            &line
        };

        // Empty/all-comment lines ignored
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (line_app_id, line_options) = line.split_once(':').ok_or_else(|| format!("Line {} of {} is not a comment and is missing a colon (:) to separate the app ID from the options", line_no, filename))?;
        let line_app_id = line_app_id.trim();

        if line_app_id != app_id {
            continue;
        }

        let line_options = line_options.trim();
        if line_options.is_empty() {
            return Ok(None);
        } else {
            return Ok(Some(line_options.to_string()));
        }
    }
    Ok(None)
}
