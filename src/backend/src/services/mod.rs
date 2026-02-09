//! Business logic services
//!
//! This module contains business logic separated from HTTP concerns.
//! Services are pure functions or stateless operations that can be
//! easily tested and reused.

pub mod auth;
pub mod certificate;
pub mod firmware;
pub mod marker;
pub mod network;
