#[macro_use]
extern crate tracing;

use colorspace::{Encoding, Format};
use host1x_buffer::PixelBuffer;
use once_cell::sync::OnceCell;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracy_client::{Client, GpuContextType, register_demangler};

register_demangler!();

static CONTEXT: OnceCell<TracingContext> = OnceCell::new();

pub trait ToTracy {
    fn upload<B: PixelBuffer<Format = Self> + AsRef<[u8]>>(client: &tracy_client::Client, buf: &B);
}
impl ToTracy for colorspace::Rgba8888 {
    fn upload<B: PixelBuffer<Format = Self> + AsRef<[u8]>>(client: &tracy_client::Client, buf: &B) {
    }
}
impl ToTracy for colorspace::Bgra8888 {
    fn upload<B: PixelBuffer<Format = Self> + AsRef<[u8]>>(client: &tracy_client::Client, buf: &B) {
        let dst_stride = buf.width() as usize
            * <<B::Format as Format>::Encoding as Encoding>::BPP.as_u8() as usize
            / 8;
        let mut dst = vec![0u8; buf.height() as usize * dst_stride];

        let planes = [(buf.stride(), buf.as_ref())];
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let pixel = Self::get_pixel(
                    x,
                    y,
                    &planes,
                    <<B::Format as Format>::Encoding as Encoding>::BPP,
                )
                .unwrap();
            }
        }
        todo!()
    }
}

pub struct TracingContext {
    tracy: tracy_client::Client,
    gr2d: tracy_client::GpuContext,
}

impl TracingContext {
    pub fn frame<B: PixelBuffer + AsRef<[u8]>>(&self, buf: &B)
    where
        B::Format: ToTracy,
    {
        /*let mut dst = vec![0u8; buf.size() as usize];
        for row in 0..buf.height() {
            let offset = row as usize * buf.stride() as usize;
            let pixels = &buf.as_ref()[offset
                ..offset
                    + buf.width() as usize
                        * <<B::Format as Format>::Encoding as Encoding>::BPP.as_u8() as usize
                        / 4];
        }

        // FIXME: need to handle format conversion image scaling
        self.tracy.frame_mark();
        self.tracy.frame_image(
            buf.as_ref(),
            buf.width().try_into().unwrap(),
            buf.height().try_into().unwrap(),
            1,
            false,
        );*/
    }
}

pub fn init() {
    tracing_subscriber::registry()
        // Put tracy layer before fmt layer which injects ansi escape sequences causing garbled span names in Tracy.
        // https://github.com/nagisa/rust_tracy_client/issues/72
        .with(tracing_tracy::TracyLayer::default())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let tracy = Client::start();
    let gr2d = tracy
        .clone()
        .new_gpu_context(Some("gr2d"), GpuContextType::Invalid, 0, 1.0)
        .unwrap();
    if CONTEXT.set(TracingContext { tracy, gr2d }).is_err() {
        panic!("tracing context already initialized")
    }
}

pub fn try_get_context() -> Option<&'static TracingContext> {
    CONTEXT.get()
}

pub fn get_context() -> &'static TracingContext {
    CONTEXT.get().expect("tracing context initialized")
}

pub unsafe fn get_context_unchecked() -> &'static TracingContext {
    unsafe { CONTEXT.get_unchecked() }
}
