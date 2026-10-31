//! TEMPORARY: dump raw blend results to reverse-engineer GR2D alpha blending.
//!
//! Only uses alpha modes that take alpha from SRCFGC or the destination, never
//! from memory planes (ALPTYPE=4 hung the engine).

use std::{collections::BTreeSet, sync::atomic::Ordering::Relaxed};

use colorspace::{Bgra8888, color::Color, encoding::Rgba8};
use gr2d::fill::probe;
use hw_tests::{
    PixelBufferExt as _,
    ctx::{Gr2d, gr2d as ctx},
    exec,
    util::rect,
};

#[tegra_test_macros::test]
async fn probe(ctx: &Gr2d) {
    // (swap: bit0 BEBSWAP / bit1 BEWSWAP, srcfgc mode)
    let modes = [(0, 2), (1, 2), (1, 3), (2, 2), (2, 3), (3, 2), (3, 3)];
    let dsts = [(0x10, 0x20, 0x30, 0x40), (0, 0, 255, 0xff)];
    let srcs = [(255, 255, 255), (255, 0, 0)];
    let alphas = [0x00u8, 0x40, 0xc0];
    probe::ALPTYPE.store(9, Relaxed);
    probe::SRCCD.store(1, Relaxed);
    probe::ALPHABLEND.store(0, Relaxed);
    for (swap, sm) in modes {
        probe::SWAP.store(swap, Relaxed);
        probe::SRCFGC_MODE.store(sm, Relaxed);
        for (dr, dg, db, da) in dsts {
            for (sr, sg, sb) in srcs {
                for a in alphas {
                    let buf = exec!(
                        ctx,
                        Bgra8888,
                        8,
                        8,
                        (
                            |buf| gr2d::clear_with_color(buf, Color::rgba(dr, dg, db, da)),
                            |buf| gr2d::fill_rect(
                                buf,
                                Color::<Rgba8>::rgba(sr, sg, sb, a),
                                rect(2, 2, 4, 4)
                            ),
                        )
                    );
                    let mut inside = BTreeSet::new();
                    for y in 2..6 {
                        for x in 2..6 {
                            inside.insert(buf.pixel_raw(x, y).to_vec());
                        }
                    }
                    eprintln!(
                        "PROBE swap={swap} sm={sm} dst[A,R,G,B]={:02x?} src={:02x?} a={a:02x} -> {:02x?} outside={:02x?}",
                        [da, dr, dg, db],
                        [sr, sg, sb],
                        inside,
                        buf.pixel_raw(0, 0),
                    );
                }
            }
        }
    }
}
