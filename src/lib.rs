//! # Watch module
//!
//! This module provides utilities to monitor remote Git branches and detect
//! new commits.
//!
//! ## Functions
//!
//! ### `watch_once`
//! Checks if a new commit is present on the tracked remote branch.
//!
//! - Compares last known local commit (`ctx.repo.last_commit`) with the remote.
//! - Returns `Some(hash)` if a new commit is detected or `force_commit` is enabled.
//! - Returns `None` if no new commit is detected.
//! - Returns `Err` if an error occurs while fetching the remote hash.

pub mod cli;
pub mod config;
pub mod core;
pub mod daemon;
pub mod exec;
pub mod git;
pub mod log;
pub mod notifications;
