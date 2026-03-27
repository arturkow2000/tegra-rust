//! Core utilities for accessing Host1x, and hardware behind the Host1x bridge.
#![cfg_attr(not(test), no_std)]
// Required for CommandSequence::relocs() implementation
#![recursion_limit = "256"]

pub mod cmd;
pub mod cmdseq;
pub use host1x_reg as reg;
pub mod pushbuf;
