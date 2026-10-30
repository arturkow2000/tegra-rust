use std::{
    io,
    marker::PhantomData,
    mem::MaybeUninit,
    os::fd::{AsFd, AsRawFd},
};

use getset::Getters;
use host1x_buffer::access::ReadWrite;
use host1x_core::cmdseq::CommandSequence;
use tracing::field;

use crate::{
    buffer::BufferType,
    device::TegraDRM,
    fence::Syncobj,
    mapping::MappingMode,
    uapi::{
        v1::buffer::GenericBuffer,
        v2::{channel::mapping::BufferMapping, syncpt::SyncPoint},
    },
};

use super::ioctl;

pub mod mapping;

#[derive(Debug, Getters)]
pub struct Channel<C: host1x_client::Client> {
    #[get = "pub"]
    dev: TegraDRM,
    context: u32,
    _client: PhantomData<C>,
}
impl<C: host1x_client::Client> Channel<C> {
    /// Open a new Host1x channel of given class.
    #[instrument(skip_all, fields(
        dev = dev.as_fd().as_raw_fd(),
        class = field::display(C::CLASS),
    ))]
    pub fn new(dev: &TegraDRM) -> io::Result<Self> {
        let info = ioctl::channel_open(dev, C::CLASS.value() as u32).inspect_err(|error| {
            error!(
                error = field::display(error),
                "failed to open host1x channel"
            )
        })?;
        debug!(
            context = info.context,
            version = info.version,
            capabilities = info.capabilities.bits(),
            "channel open"
        );
        Ok(Channel {
            dev: dev.clone(),
            context: info.context,
            _client: PhantomData,
        })
    }
}
impl<C: host1x_client::Client> Channel<C> {
    pub fn map_buffer<'a, CMM: MappingMode, T: BufferType>(
        &'a self,
        buffer: GenericBuffer<CMM, T>,
    ) -> io::Result<BufferMapping<'a, C, T, CMM, ReadWrite>> {
        BufferMapping::new(self, buffer)
    }

    pub(crate) fn submit(
        &self,
        seq: impl CommandSequence<C>,
        sp: &SyncPoint,
        syncobj_in: Option<&mut Syncobj>,
        syncobj_out: Option<&mut Syncobj>,
    ) -> io::Result<u32> {
        // FIXME: need to unify relocations definition between host1x-drm, host1x-nvhost and host1x-core.
        let relocs: Vec<_> = seq
            .relocs()
            .map(|r| ioctl::drm_tegra_submit_buf {
                mapping: r.mapping,
                flags: 0,
                reloc_target_offset: r.buf_offset,
                reloc_gather_offset_words: r.patch_offset,
                reloc_shift: r.shift,
            })
            .collect();
        // TODO: https://github.com/rust-lang/rust/issues/76560 would allow to allocate buf and relocs on the stack.
        // in fact that compiles even now with `#![feature(generic_const_exprs)]`, but it's marked as incomplete and I want
        // to avoid using things which are likely to introduce unsoundness into safe code.
        let mut buf = vec![MaybeUninit::uninit(); seq.words().len()];
        let buf = seq.write_to_slice(&mut buf[..]);

        let fence = unsafe {
            ioctl::channel_submit(
                &self.dev,
                self.context,
                sp.id(),
                1,
                buf,
                &relocs,
                syncobj_in.map(|sobj| sobj.handle()),
                syncobj_out.map(|sobj| sobj.handle()),
            )?
        };
        Ok(fence)
    }
}
impl<C: host1x_client::Client> host1x_core::queue::Channel for Channel<C> {
    type Client = C;
}
impl<C: host1x_client::Client> Drop for Channel<C> {
    fn drop(&mut self) {
        let span = debug_span!(
            "drop",
            dev.fd = self.dev.as_fd().as_raw_fd(),
            channel.context = self.context,
            class = C::CLASS.value(),
        );
        let _guard = span.enter();
        debug!("closing host1x channel");

        let r = ioctl::channel_close(&self.dev, self.context);
        #[cfg(debug_assertions)]
        r.unwrap();
    }
}
