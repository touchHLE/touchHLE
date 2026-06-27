/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `MapKit.framework/MapKit`.
//!
//! On iOS, MapKit provides `MKMapView`, `MKAnnotation`, geocoding etc. Only
//! a very small subset of touchHLE's target apps use it for actual map
//! rendering — but a much larger set lists it as a Mach-O dependency
//! because they pull in some umbrella header (or because their UI framework
//! does so transitively).
//!
//! Without a [crate::dyld::HostDylib] entry for the path
//! `/System/Library/Frameworks/MapKit.framework/MapKit`, touchHLE prints a
//! `Warning: app binary depends on unimplemented or missing dylib …` at
//! startup, which can spook users into reporting otherwise-fine apps as
//! broken (e.g. HyperHLE appdb report #52, Pumpkins vs Monsters — which
//! never actually opens a map view, it only links MapKit).
//!
//! This stub exists so the dependency is recognized and the warning is
//! suppressed. Real map rendering is not implemented; specific MapKit
//! classes/functions can be added here as they come up.

use crate::dyld::FunctionExports;

pub const FUNCTIONS: FunctionExports = &[];
