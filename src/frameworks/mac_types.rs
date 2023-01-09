//! `MacTypes.h`
//!
//! It's unclear if this belongs to some particular "framework", but it is
//! definitely from Carbon.

/// Status code. At least in Audio Toolbox's use, this is usually a FourCC.
/// 0 means success.
pub type OSStatus = i32;
