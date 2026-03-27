#[macro_use]
extern crate tracing;

use std::{mem::MaybeUninit, num::NonZeroU32};

use colorspace::{Rgba8888, color::Color};
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};
use gr2d::Gr2d;
use host1x_core::{cmd::WriteNoIncr, pushbuf::PushBuffer, reg::class::INCR_SYNCPT};
use host1x_drm::{
    buffer::PixelBuffer,
    device::TegraDRM,
    uapi::{
        v1::buffer::Buffer,
        v2::{channel::Channel, syncpt::SyncPoint},
    },
};

fn main() {
    let _guard = instrumentation::init();
    info!("doing some GR2D tests");

    let gpu = TegraDRM::open("/dev/dri/card1").unwrap();
    let mut buf = Buffer::<PixelBuffer<Rgba8888>>::new(
        &gpu,
        NonZeroU32::new(16).unwrap(),
        NonZeroU32::new(16).unwrap(),
    )
    .unwrap()
    .mmap_rw()
    .unwrap();
    let sync = SyncPoint::alloc(&gpu).unwrap();
    let gr2d = Channel::new(&gpu).unwrap();
    struct X {
        ch: Channel<gr2d_reg::Client>,
        sync: SyncPoint,
    }
    impl Gr2d for X {
        unsafe fn exec_blocking<C: host1x_core::cmdseq::CommandSequence<gr2d_reg::Client>>(
            &self,
            cmdseq: C,
        ) {
            let mut cmdstor = vec![MaybeUninit::uninit(); C::N_WORDS + 2];
            let mut relstor = vec![MaybeUninit::uninit(); C::N_RELOCS];
            let mut pb = PushBuffer::new(&mut cmdstor[..], &mut relstor[..]);
            pb.push(cmdseq);
            pb.push((WriteNoIncr::new([
                INCR_SYNCPT::COND::OP_DONE + INCR_SYNCPT::INDX.val(self.sync.id())
            ]),));

            unsafe {
                self.ch.exec_pushbuf_blocking(&pb, &self.sync).unwrap();
            }
        }
    }
    let gr2d = X { ch: gr2d, sync };
    let mut mapping = gr2d.ch.map_buffer(&mut buf).unwrap();

    gr2d.clear_with_color(&mut mapping, Color::rgb(255, 0, 0));
    gr2d.fill_rect(
        &mut mapping,
        Color::rgb(0, 255, 0),
        Rectangle {
            top_left: Point::new(10, 10),
            size: Size::new(10, 10),
        },
    );
    warn!("after: {:#x?}", &mapping[..16]);
    instrumentation::get_context().frame(mapping.buffer());
}
