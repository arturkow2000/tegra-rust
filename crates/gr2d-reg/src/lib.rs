//! Register definitions for Tegra's GR2D (2D accelerator) hardware.
#![no_std]

use arbitrary_int::u10;
use host1x_reg::impl_registers;

#[derive(Debug, Clone, Copy)]
pub struct Client;
impl host1x_client::Client for Client {
    const CLASS: u10 = u10::new(0x51);
}

impl_registers![
    crate::Client,
    pub TRIGGER0 OFFSET(0x09) RELOCATABLE(false) [
        TRIGGER OFFSET(0) NUMBITS(16) []
    ],
    pub TRIGGER1 OFFSET(0x0a) RELOCATABLE(false) [
        TRIGGER OFFSET(0) NUMBITS(16) []
    ],
    pub TRIGGER2 OFFSET(0x0b) RELOCATABLE(false) [
        TRIGGER OFFSET(0) NUMBITS(16) []
    ],
    pub CMDSEL OFFSET(0x0c) RELOCATABLE(false) [
        PRIORITY OFFSET(28) NUMBITS(1) [
            LOW = 0b0,
            HIGH = 0b1,
        ],
        LINKVAL OFFSET(25) NUMBITS(3) [],
        LINKEN OFFSET(24) NUMBITS(1) [],
        BUFFER_INDEX OFFSET(16) NUMBITS(8) [],
        FRAME_END OFFSET(15) NUMBITS(1) [],
        FRAME_START OFFSET(14) NUMBITS(1) [],
        CLIP_SOURCE_TOP_BOTTOM OFFSET(10) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        OUTPUT OFFSET(8) NUMBITS(2) [
            MEMORY = 0,
            EPP = 1,
        ],
        CBSBDISABLE OFFSET(7) NUMBITS(1) [],
        HOSTTRIGGER OFFSET(6) NUMBITS(1) [],
        VITRIGGER OFFSET(5) NUMBITS(1) [],
        CBENABLE OFFSET(4) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        SBOR2D OFFSET(0) NUMBITS(1) [
            G2 = 0,
            SB = 1,
        ]
    ],
    pub SBFMT0 OFFSET(0x1c) RELOCATABLE(false) [
        DIFMT OFFSET(12) NUMBITS(5) [
            RGB565 = 0b01000,
            RGBA8888 = 0b01110,
            BGRA8888 = 0b01111,
        ],
        SIFMT OFFSET(0) NUMBITS(5) [
            RGB565 = 0b01000,
            RGBA8888 = 0b01110,
            BGRA8888 = 0b01111,
        ]
    ],
    pub CTRL0 OFFSET(0x1d) RELOCATABLE(false) [
        ENDITH OFFSET(30) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        KPOL OFFSET(28) NUMBITS(1) [
            WITHIN_BOUNDS = 0,
            OUTSIDE_BOUNDS = 1,
        ],
        KEYEN OFFSET(27) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        DBSEL OFFSET(26) NUMBITS(1) [
            DST_A = 0,
            DST_B = 1,
        ],
        SBSEL OFFSET(25) NUMBITS(1) [
            SRC_A = 0,
            SRC_B = 1,
        ],
        SITYPE OFFSET(24) NUMBITS(1) [
            TOP_FIELD = 0,
            BOTTOM_FIELD = 1,
        ],
        RANGEREDFRM OFFSET(23) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        HFTYPE OFFSET(20) NUMBITS(3) [
            INTERP = 0,
            LPF1 = 1,
            LPF2 = 2,
            LPF3 = 3,
            LPF4 = 4,
            LPF5 = 5,
            LPF6 = 6,
            DISABLE = 7,
        ],
        DISCSC OFFSET(19) NUMBITS(1) [
            ENABLE = 0,
            DISABLE = 1,
        ],
        VFEN OFFSET(18) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        VFTYPE OFFSET(16) NUMBITS(2) [
            INTERP = 0,
            AVG25_INTERP75 = 1,
            AVG50_INTERP50 = 2,
            AVG = 3,
        ],
        UVST OFFSET(8) NUMBITS(9) [
            UVS2X = 0,
            UVS1X = 1,
            UVS4X = 2,
            UVS_G2UVSTRIDE = 3,
        ],
        ENAHF OFFSET(7) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        ENAVF OFFSET(6) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        IMODE OFFSET(5) NUMBITS(1) [
            MULTIPLEX = 0,
            PLANAR = 1,
        ]
    ],
    pub CTRL1 OFFSET(0x1e) RELOCATABLE(false) [
        FR_TYPE OFFSET(26) NUMBITS(3) [
            FLIP_X = 0,
            FLIP_Y = 1,
            TRANS_LR = 2,
            TRANS_RL = 3,
            ROT_90 = 4,
            ROT_180 = 5,
            ROT_270 = 6,
            IDENTITY = 7,
        ],
        FR_MODE OFFSET(24) NUMBITS(2) [
            DISABLE = 0,
            SRC_DST_COPY = 1,
            SQUARE = 2,
            BLANK = 3,
        ],
        CLIPC OFFSET(21) NUMBITS(2) [
            DISABLE = 0,
            DRAW_INSIDE = 2,
            DRAW_OUTSIDE = 3,
        ],
        ALPSRCORDST OFFSET(9) NUMBITS(1) [
            SRC = 0,
            DST = 1,
        ],
        ALPTYPE OFFSET(4) NUMBITS(5) [
            FIX = 0,
            PL1BPP = 1,
            PL2BPP = 2,
            PL4BPP = 3,
            PL8BPP = 4,
            PL44BPP = 5,
            PLS1BPP = 6,
            PLS4BPPAL = 7,
            PLS4BPP = 8,
            PLS8BPP = 9,
            PLS8BX = 10,
            PLS1BPPAL = 11,
        ],
        BEWSWAP OFFSET(3) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        BEBSWAP OFFSET(2) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        BITSWAP OFFSET(1) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ]
    ],
    pub CTRLMAIN OFFSET(0x1f) RELOCATABLE(false) [
        PATSEL OFFSET(30) NUMBITS(1) [
            SCREEN = 0,
            MEMORY = 1,
        ],
        DSTDIR OFFSET(28) NUMBITS(1) [],
        SRCDIR OFFSET(27) NUMBITS(1) [],
        GCSW OFFSET(25) NUMBITS(2) [
            DISABLE = 0,
        ],
        SRCBAS OFFSET(24) NUMBITS(1) [
            SRCBA = 0,
            DSTBA = 1,
        ],
        SRCT OFFSET(22) NUMBITS(2) [],
        HLMONO OFFSET(21) NUMBITS(1) [],
        SRCCD OFFSET(20) NUMBITS(1) [],
        DSTT OFFSET(18) NUMBITS(2) [],
        DSTCD OFFSET(16) NUMBITS(2) [
            BPP8 = 0,
            BPP16 = 1,
            BPP32 = 2,
        ],
        SRCSEL OFFSET(15) NUMBITS(1) [
            SCREEN = 0,
            MEMORY = 1,
        ],
        YFLIP OFFSET(14) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        PATPACK OFFSET(13) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        SRCPACK OFFSET(12) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        XYTDW OFFSET(11) NUMBITS(1) [],
        YDIR OFFSET(10) NUMBITS(1) [
            INCREMENTING = 0,
            DECREMENTING = 1,
        ],
        XDIR OFFSET(9) NUMBITS(1) [
            INCREMENTING = 0,
            DECREMENTING = 1,
        ],
        PATFL OFFSET(8) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        PATSLD OFFSET(7) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        SRCSLD OFFSET(6) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        ALPEN OFFSET(5) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        FADEN OFFSET(4) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        TEST0BIT OFFSET(3) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        TURBOFILL OFFSET(2) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        CMDT OFFSET(0) NUMBITS(2) [
            BITBLT = 0,
            LINEDRAW = 1,
            VCAA = 2,
        ]
    ],
    pub ROPFADE OFFSET(0x20) RELOCATABLE(false) [
        FADOFF OFFSET(24) NUMBITS(8) [],
        FADCOE OFFSET(16) NUMBITS(8) [],
        ROP OFFSET(0) NUMBITS(8) [
            COPY = 0xcc
        ],
    ],
    pub ALPHABLEND OFFSET(0x21) RELOCATABLE(false) [
        ALPHA1V OFFSET(24) NUMBITS(8) [],
        ALPHA0V OFFSET(16) NUMBITS(8) [],
        ALPHAINV OFFSET(8) NUMBITS(8) [],
        ALPHA OFFSET(0) NUMBITS(8) [],
    ],
    pub CLIPLEFTTOP OFFSET(0x22) RELOCATABLE(false) [
        CLIPT OFFSET(16) NUMBITS(15) [],
        CLIPL OFFSET(0) NUMBITS(15) [],
    ],
    pub CLIPRIGHTBOT OFFSET(0x23) RELOCATABLE(false) [
        CLIPB OFFSET(16) NUMBITS(15) [],
        CLIPR OFFSET(0) NUMBITS(15) [],
    ],
    pub PATPACK OFFSET(0x24) RELOCATABLE(false) [
        PATLNGAP OFFSET(0) NUMBITS(4) [],
    ],
    pub PATPACK_SIZE OFFSET(0x25) RELOCATABLE(false) [
        PATMONOH OFFSET(16) NUMBITS(16) [],
        PATMONOW OFFSET(0) NUMBITS(16) [],
    ],
    pub PATBA OFFSET(0x26) RELOCATABLE(true) [],
    pub PATOS OFFSET(0x27) RELOCATABLE(false) [
        PATYO OFFSET(28) NUMBITS(4) [],
        PATXO OFFSET(24) NUMBITS(4) [],
        PATT OFFSET(21) NUMBITS(2) [],
        PATCD OFFSET(16) NUMBITS(1) [],
        PATST OFFSET(0) NUMBITS(16) [],
    ],
    pub PATBGC OFFSET(0x28) RELOCATABLE(false) [
        PATBGC OFFSET(0) NUMBITS(32) [],
    ],
    pub PATFGC OFFSET(0x29) RELOCATABLE(false) [
        PATFGC OFFSET(0) NUMBITS(32) [],
    ],
    pub PATKEY OFFSET(0x2a) RELOCATABLE(false) [
        PATKEY OFFSET(0) NUMBITS(32) [],
    ],
    pub DSTBA OFFSET(0x2b) RELOCATABLE(true) [],
    pub DSTBA_B OFFSET(0x2c) RELOCATABLE(true) [],
    pub DSTBA_C OFFSET(0x2d) RELOCATABLE(true) [],
    pub DSTST OFFSET(0x2e) RELOCATABLE(false) [
        DSTS OFFSET(0) NUMBITS(16) [],
    ],
    pub SRCPACK OFFSET(0x2f) RELOCATABLE(false) [
        SRCLNGAP OFFSET(0) NUMBITS(4) [],
    ],
    pub SRCPACK_SIZE OFFSET(0x30) RELOCATABLE(false) [
        SRCMONOH OFFSET(16) NUMBITS(16) [],
        SRCMONOW OFFSET(0) NUMBITS(16) [],
    ],
    pub SRCBA OFFSET(0x31) RELOCATABLE(true) [],
    pub SRCBA_B OFFSET(0x32) RELOCATABLE(true) [],
    pub SRCST OFFSET(0x33) RELOCATABLE(false) [
        SRCS OFFSET(0) NUMBITS(16) [],
    ],
    pub SRCBGC OFFSET(0x34) RELOCATABLE(false) [
        SRCBGC OFFSET(0) NUMBITS(32) [],
    ],
    pub SRCFGC OFFSET(0x35) RELOCATABLE(false) [
        SRCFGC OFFSET(0) NUMBITS(32) [],
    ],
    pub SRCKEY OFFSET(0x36) RELOCATABLE(false) [
        SRCKEY OFFSET(0) NUMBITS(32) [],
    ],
    pub SRCSIZE OFFSET(0x37) RELOCATABLE(false) [
        SRCHEIGHT OFFSET(16) NUMBITS(16) [],
        SRCWIDTH OFFSET(0) NUMBITS(16) [],
    ],
    pub DSTSIZE OFFSET(0x38) RELOCATABLE(false) [
        DSTHEIGHT OFFSET(16) NUMBITS(16) [],
        DSTWIDTH OFFSET(0) NUMBITS(16) [],
    ],
    pub SRCPS OFFSET(0x39) RELOCATABLE(false) [
        SRCY OFFSET(16) NUMBITS(16) [],
        SRCX OFFSET(0) NUMBITS(16) [],
    ],
    pub DSTPS OFFSET(0x3a) RELOCATABLE(false) [
        DSTY OFFSET(16) NUMBITS(16) [],
        DSTX OFFSET(0) NUMBITS(16) [],
    ],
    pub CBDES OFFSET(0x3b) RELOCATABLE(false) [
        TOPCLIP OFFSET(31) NUMBITS(1) [
            DISABLE = 0,
            ENABLE = 1,
        ],
        CBLINE OFFSET(16) NUMBITS(15) [],
        CBCOUNT OFFSET(0) NUMBITS(8) [],
    ],
    pub CBSTRIDE OFFSET(0x3c) RELOCATABLE(false) [
        CBUVSTRIDE OFFSET(30) NUMBITS(2) [
            CBS2X = 0,
            CBS1X = 1,
            CBS4X = 2,
        ],
        CBSTRIDE OFFSET(0) NUMBITS(24) []
    ],
    pub LINESETTING OFFSET(0x3d) RELOCATABLE(false) [
        OCTANT OFFSET(29) NUMBITS(3) [],
        LINEUSEOCTANT OFFSET(28) NUMBITS(1) [],
        DROPLASTP OFFSET(27) NUMBITS(1) [],
        LINEYDIR OFFSET(26) NUMBITS(1) [],
        LINEXDIR OFFSET(25) NUMBITS(1) [],
        MAJOR OFFSET(24) NUMBITS(1) [
            XMAJOR = 0,
            YMAJOR = 1,
        ],
        GAMMA OFFSET(0) NUMBITS(21) [],
    ],
    pub LINEDELTAN OFFSET(0x3e) RELOCATABLE(false) [
        DELTAN OFFSET(0) NUMBITS(21) [],
    ],
    pub LINEDELTAM OFFSET(0x3f) RELOCATABLE(false) [
        DELTAM OFFSET(0) NUMBITS(21) [],
    ],
    pub LINEPOS OFFSET(0x40) RELOCATABLE(false) [
        LINEYPOS OFFSET(16) NUMBITS(16) [],
        LINEXPOS OFFSET(0) NUMBITS(16) [],
    ],
    pub LINELEN OFFSET(0x41) RELOCATABLE(false) [
        LINELEN OFFSET(0) NUMBITS(15) [],
    ],
    pub SRCST_B OFFSET(0x43) RELOCATABLE(false) [
        SRCS_B OFFSET(0) NUMBITS(16) [],
    ],
    pub UVSTRIDE OFFSET(0x44) RELOCATABLE(false) [
        UVSTRIDE OFFSET(0) NUMBITS(16) [],
    ],
    pub CBDES2 OFFSET(0x45) RELOCATABLE(false) [
        TOP_CBLINE OFFSET(0) NUMBITS(15) [],
    ],
    pub TILEMODE OFFSET(0x46) RELOCATABLE(false) [
        DST_WR_TILE_MODE OFFSET(20) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
        DST_RD_TILE_MODE OFFSET(16) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
        PAT_UV_TILE_MODE OFFSET(12) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
        PAT_Y_TILE_MODE OFFSET(8) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
        SRC_UV_TILE_MODE OFFSET(4) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
        SRC_Y_TILE_MODE OFFSET(0) NUMBITS(1) [
            LINEAR = 0,
            TILED = 1,
        ],
    ],
    pub PATBASE OFFSET(0x47) RELOCATABLE(true) [],
];
