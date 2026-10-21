//! Core utilities for accessing Host1x, and hardware behind the Host1x bridge.
#![cfg_attr(not(test), no_std)]
// Required for CommandSequence::relocs() implementation
#![recursion_limit = "256"]
// Required by job!() to create the brand with `super let`.
#![feature(allow_internal_unstable)]
#![allow(internal_features)]

pub mod cmd;
pub mod cmdseq;
pub mod job;
pub use host1x_reg as reg;
pub mod queue;
pub mod task;
