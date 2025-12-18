//! Domain-based type organization
//!
//! Types are organized by domain to match the structure in `update/`:
//! - auth: Authentication types
//! - device: Device operation state
//! - factory_reset: Factory reset types
//! - network: Network configuration types
//! - update: Firmware update types
//! - common: Shared system types

pub mod auth;
pub mod common;
pub mod device;
pub mod factory_reset;
pub mod network;
pub mod update;

// Re-export all types for backward compatibility
pub use auth::*;
pub use common::*;
pub use device::*;
pub use factory_reset::*;
pub use network::*;
pub use update::*;
