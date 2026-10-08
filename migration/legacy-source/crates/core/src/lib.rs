//! SCOPENET launcher engine.
//!
//! The Tauri app is a thin UI shell around this crate: everything that
//! downloads, installs or launches Minecraft lives here so it can be unit
//! tested without a window.

pub mod assets;
pub mod authlib;
pub mod http;
pub mod install;
pub mod java;
pub mod launch;
pub mod libraries;
pub mod loaders;
pub mod maven;
pub mod meta;
pub mod options;
pub mod paths;
pub mod ping;
pub mod progress;
pub mod rules;
pub mod servers_dat;
pub mod sync;
pub mod textures;
pub mod sys;
pub mod version;

pub use paths::Layout;
pub use progress::{Event, Reporter, Stage};
