//! Feature numbering system for tracking and managing feature identifiers.
//!
//! This module provides a centralized system for assigning and tracking
//! unique feature numbers across the application.

pub mod api;
pub mod data_models;
pub mod error;
pub mod storage;

pub use api::*;
pub use data_models::*;
pub use error::FeatureNumberingError;
pub use storage::FeatureStorage;