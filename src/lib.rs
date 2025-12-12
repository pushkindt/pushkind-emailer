//! Core library for the pushkind emailer application.
//!
//! This crate exposes the domain types, data models and utilities that power
//! the pushkind emailer service.  The binary in [`main`](../main.rs) builds on
//! top of these modules to provide an HTTP server and background workers.

pub mod domain;
pub mod forms;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;
pub mod utils;

pub const SERVICE_ACCESS_ROLE: &str = "emailer";
