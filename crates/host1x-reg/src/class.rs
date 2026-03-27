//! Host1x class methods.

use crate::impl_registers;

// TODO: perhabs this would be better expressed to high-level sync APIs instead
// of raw register writes.

impl_registers![
    any,
    pub INCR_SYNCPT OFFSET(0x00) RELOCATABLE(false) [
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
