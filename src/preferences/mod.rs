//! User preference management for model selection.
//!
//! This module provides tools for learning and applying user-specific
//! preferences to optimize model selection over time.

pub mod manager;
pub mod learning;
pub mod models;

pub use manager::*;
pub use learning::*;
pub use models::*;