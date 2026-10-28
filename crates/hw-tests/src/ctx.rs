use std::{
    cmp::min,
    num::NonZeroU32,
    sync::atomic::{AtomicUsize, Ordering},
};

use colorspace::Format;
use host1x_buffer::access::ReadWrite;
use host1x_core::{cmdseq::CommandSequence, job::Job, queue::Queue as _};
use host1x_drm::{
    buffer::PixelBuffer,
    device::TegraDRM,
    fence::Syncobj,
    uapi::{
        v1::buffer::{Buffer, RwBuffer},
        v2::{
            channel::{Channel, mapping::BufferMapping},
            queue::Queue,
            syncpt::SyncPoint,
        },
    },
};
use once_cell::sync::OnceCell;

// Hw limits for Tegra 20
// TODO: move this elsewhere, this crate should not depend on specific hw revision
const MAX_SYNCPOINTS: usize = 32;
const MAX_CHANNELS: usize = 8;

static DRM: OnceCell<TegraDRM> = OnceCell::new();
static GR2D: OnceCell<Gr2d> = OnceCell::new();

pub struct Gr2d {
    queues: Vec<Queue<gr2d_reg::Client>>,
    next_queue: AtomicUsize,
}

impl Gr2d {
    fn init(dev: &TegraDRM) -> Self {
        // Kernel allows us to create seemingly arbitrary number of channels (tested up to 1024),
        // but limited number of syncpoints (27 on 6.18.21, and yes, currently that means we may
        // run a very limited amount of hw-accelerated apps - syncpoint is required to execute anything).
        //
        // Currently we as many queues as we can with unique hw channel and hw syncpoint, limited by hw,
        // even if kernel allows more, its hw that limits concurrently.
        // This hasn't been for tested for being optimal and may be changed in the future.
        let queues: Vec<_> = (0..min(MAX_CHANNELS, MAX_SYNCPOINTS))
            .map(|_| Channel::<gr2d_reg::Client>::new(&dev))
            .take_while(Result::is_ok)
            .flat_map(|r| r)
            .flat_map(|ch| SyncPoint::alloc(&dev).map(|sp| (ch, sp)))
            .map(|(ch, sp)| Queue::new(ch, sp))
            .collect();
        assert!(!queues.is_empty(), "could not allocate any GR2D queue");

        Self {
            queues,
            next_queue: AtomicUsize::new(0),
        }
    }

    /// Get next queue from the list, round-robin.
    pub fn queue(&self) -> &Queue<gr2d_reg::Client> {
        let idx = self.next_queue.fetch_add(1, Ordering::Relaxed) % self.queues.len();
        &self.queues[idx]
    }

    pub fn buffer<'q, F: Format>(
        &self,
        queue: &'q Queue<gr2d_reg::Client>,
        width: u32,
        height: u32,
    ) -> BufferMapping<'q, gr2d_reg::Client, PixelBuffer<F>, ReadWrite, ReadWrite> {
        let buffer = Buffer::<PixelBuffer<F>>::new(
            drm(),
            NonZeroU32::new(width).expect("width > 0"),
            NonZeroU32::new(height).expect("height > 0"),
        )
        .expect("create GEM buffer")
        .mmap_rw()
        .map_err(|(e, _)| e)
        .expect("mmap buffer read-write");
        queue
            .channel()
            .map_buffer(buffer)
            .expect("map buffer to channel")
    }

    pub async fn exec<F: Format, S: CommandSequence<gr2d_reg::Client>>(
        &self,
        queue: &Queue<gr2d_reg::Client>,
        job: Job<
            '_,
            gr2d_reg::Client,
            BufferMapping<'_, gr2d_reg::Client, PixelBuffer<F>, ReadWrite, ReadWrite>,
            S,
        >,
    ) -> RwBuffer<PixelBuffer<F>> {
        let mut fence = Syncobj::new(drm()).expect("create syncobj");
        let pending = queue.submit(&mut fence, job).await.expect("submit job");
        pending
            .await
            .expect("await job completion")
            .unmap()
            .expect("unmap buffer from channel")
    }
}

pub fn drm() -> &'static TegraDRM {
    DRM.get_or_init(|| {
        instrumentation::init_for_testing();
        // TODO: should scan for available GPUs and open first tegra with matching driver.
        // On Tambora this is card1 because of simpledrm usage for boot graphics.
        // On typical distro it will be card0.
        TegraDRM::open("/dev/dri/card1").expect("GPU open")
    })
}

pub fn gr2d() -> &'static Gr2d {
    GR2D.get_or_init(|| Gr2d::init(drm()))
}

// For compatibility with rstest
pub mod gr2d {
    pub fn default() -> &'static super::Gr2d {
        super::gr2d()
    }
}
