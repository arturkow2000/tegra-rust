#[macro_use]
extern crate tracing;

use std::num::NonZeroU32;

use colorspace::{Bgra8888, color::Color};
use embedded_graphics::{
    prelude::{Point, Size},
    primitives::Rectangle,
};
use host1x_core::{job, queue::Queue as _};
use host1x_drm::{
    buffer::PixelBuffer,
    device::TegraDRM,
    fence::Syncobj,
    uapi::{
        v1::buffer::Buffer,
        v2::{channel::Channel, queue::Queue, syncpt::SyncPoint},
    },
};

#[tokio::main]
async fn main() {
    let _guard = instrumentation::init();
    info!("doing some GR2D tests");

    let gpu = TegraDRM::open("/dev/dri/card1").unwrap();
    let buf = Buffer::<PixelBuffer<Bgra8888>>::new(
        &gpu,
        NonZeroU32::new(256).unwrap(),
        NonZeroU32::new(256).unwrap(),
    )
    .unwrap()
    .mmap_rw()
    .unwrap();

    let sync = SyncPoint::alloc(&gpu).unwrap();
    let queue: Queue<gr2d_reg::Client> = Queue::new(Channel::new(&gpu).unwrap(), sync);
    let mapping = queue.channel().map_buffer(buf).unwrap();

    info!("before: {:x?}", &mapping[0..4]);

    let mut fence = Syncobj::new(&gpu).unwrap();
    let job = job!(
        mapping,
        (
            |buf| gr2d::clear_with_color(buf, Color::rgb(255, 0, 0)),
            |buf| {
                gr2d::fill_rect(
                    buf,
                    Color::rgb(0, 255, 0),
                    Rectangle {
                        top_left: Point::new(10, 10),
                        size: Size::new(100, 100),
                    },
                )
            },
        ),
    );
    let mapping = queue.submit(&mut fence, job).await.unwrap().await.unwrap();
    info!("after: {:x?}", &mapping[0..4]);
    instrumentation::get_context().frame(mapping.buffer());
}
