//! Core library for serialport-api.
//!
//! The rewrite will move protocol parsing, serial connection management, and
//! HTTP API behavior into testable modules here.

pub mod api;
pub mod error;
pub mod protocol;
