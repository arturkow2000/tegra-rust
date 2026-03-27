#[macro_use]
extern crate tracing;

use host1x_buffer::PixelBuffer;
use once_cell::sync::OnceCell;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracy_client::{Client, GpuContextType, register_demangler};

register_demangler!();

static CONTEXT: OnceCell<TracingContext> = OnceCell::new();

pub struct TracingContext {
    tracy: tracy_client::Client,
    gr2d: tracy_client::GpuContext,
}

impl TracingContext {
    pub fn frame(&self, buf: &(impl PixelBuffer + AsRef<[u8]>)) {
        // FIXME: need to handle format conversion image scaling
        self.tracy.frame_mark();
        self.tracy.frame_image(
            buf.as_ref(),
            buf.width().try_into().unwrap(),
            buf.height().try_into().unwrap(),
            1,
            false,
        );
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
