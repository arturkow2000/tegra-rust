//! Host1x class methods.

use crate::impl_registers;

// TODO: perhabs this would be better expressed to high-level sync APIs instead
// of raw register writes.

impl_registers![
    any,
    /// Increment a syncpoint.
    ///
    /// # Safety
    /// Syncpoints back the fences of jobs: once a syncpoint reaches the value expected
    /// for a job, the job counts as done and its buffers are given back to the CPU.
    /// Every write increments syncpoint `INDX` once condition `COND` is met. The caller
    /// must ensure that each such increment targets a syncpoint owned by the caller, and
    /// is accounted for in the number of increments declared for the job when submitting
    /// it, so that no job fence signals before the GPU is done.
    pub INCR_SYNCPT OFFSET(0x00) RELOCATABLE(false) UNSAFE(true) [
        COND OFFSET(8) NUMBITS(8) [
            IMMEDIATE = 0,
            OP_DONE = 1,
            RD_DONE = 2,
            REG_WR_SAFE = 3,
            COND_4 = 4,
            COND_5 = 5,
            COND_6 = 6,
            COND_7 = 7,
            COND_8 = 8,
            COND_9 = 9,
            COND_10 = 10,
            COND_11 = 11,
            COND_12 = 12,
            COND_13 = 13,
            COND_14 = 14,
            COND_15 = 15,
        ],
        INDX OFFSET(0) NUMBITS(8) []
    ]
];
