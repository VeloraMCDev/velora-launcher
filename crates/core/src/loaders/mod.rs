//! Mod loader installation. Each loader produces a child version JSON that
//! is merged on top of vanilla.

pub mod fabric;
pub mod forge;

#[cfg(test)]
mod compatibility_tests;
