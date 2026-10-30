use std::{
    io,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    os::fd::{AsFd, AsRawFd},
    ptr,
};

use crate::{
    buffer::BufferType,
    mapping::{Mapped, MappingMode},
    uapi::{v1::buffer::GenericBuffer, v2::ioctl},
};

use super::Channel;
use colorspace::Format;
use host1x_buffer::{Buffer, Layout, PixelLayout};
use host1x_core::job::{MappedTo, Mapping};

#[derive(Debug)]
pub struct BufferMapping<'a, C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped>
{
    channel: &'a Channel<C>,
    buffer: ManuallyDrop<GenericBuffer<CMM, T>>,
    mapping_id: ManuallyDrop<MM::Handle<u32>>,
}
impl<'a, C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped>
    BufferMapping<'a, C, T, CMM, MM>
{
    pub(super) fn new(channel: &'a Channel<C>, buffer: GenericBuffer<CMM, T>) -> io::Result<Self> {
        // Must be enforced, GEM handles are local to DRM fd, and may trigger UB
        // when handles created from different device instances are mixed up.
        assert_eq!(
            channel.dev.as_fd().as_raw_fd(),
            buffer.dev.as_fd().as_raw_fd(),
            "cannot map buffer to a channel open on another instance of the device"
        );
        match MM::create_host1x_mapping(|flags| {
            ioctl::channel_map(&channel.dev, channel.context, buffer.handle, flags)
        }) {
            Ok(mapping_id) => Ok(Self {
                channel,
                buffer: ManuallyDrop::new(buffer),
                mapping_id: ManuallyDrop::new(mapping_id),
            }),
            Err(error) => Err(error),
        }
    }

    /// Opaque identifier of the mapping, valid within the channel.
    pub fn mapping_id(&self) -> u32 {
        *MM::get_handle_ref(&self.mapping_id)
    }

    pub fn buffer(&self) -> &GenericBuffer<CMM, T> {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut GenericBuffer<CMM, T> {
        &mut self.buffer
    }

    /// Unmap buffer from the channel.
    pub fn unmap(self) -> io::Result<GenericBuffer<CMM, T>> {
        let mut this = ManuallyDrop::new(self);
        // Safety: self is wrapped in ManuallyDrop to prevent destructor from running.
        let mapping_id = unsafe { MM::get_handle(ManuallyDrop::take(&mut this.mapping_id)) };
        let buffer = unsafe { ManuallyDrop::take(&mut this.buffer) };
        let channel = this.channel;

        ioctl::channel_unmap(&channel.dev, channel.context, mapping_id)?;
        Ok(buffer)
    }
}
impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> Drop
    for BufferMapping<'_, C, T, CMM, MM>
{
    fn drop(&mut self) {
        let mapping_id = unsafe { MM::get_handle(ManuallyDrop::take(&mut self.mapping_id)) };
        let r = ioctl::channel_unmap(&self.channel.dev, self.channel.context, mapping_id);
        #[cfg(debug_assertions)]
        r.unwrap();
    }
}
// Allow dereference into inner buffer, this way may still read/write the buffer from CPU,
// even while the buffer is mapped to Host1x. This is safe, because the GPU won't actually
// touch our data unless a job is running. Other APIs are responsible for preventing
// racy access to same data.
impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> Deref
    for BufferMapping<'_, C, T, CMM, MM>
{
    type Target = GenericBuffer<CMM, T>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> DerefMut
    for BufferMapping<'_, C, T, CMM, MM>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> Layout
    for BufferMapping<'_, C, T, CMM, MM>
{
    fn size(&self) -> u32 {
        self.buffer.size()
    }
}
impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> Buffer
    for BufferMapping<'_, C, T, CMM, MM>
{
    type Desc = T::Desc;

    fn desc(&self) -> T::Desc {
        self.buffer.desc()
    }
}
impl<C: host1x_client::Client, CMM: MappingMode, MM: Mapped, F: Format> PixelLayout
    for BufferMapping<'_, C, crate::buffer::PixelBuffer<F>, CMM, MM>
{
    type Format = F;

    fn width(&self) -> u32 {
        self.buffer.width()
    }

    fn height(&self) -> u32 {
        self.buffer.height()
    }

    fn stride(&self) -> u32 {
        self.buffer.stride()
    }
}
// Safety:
// - mapping lives as long as BufferMapping does.
// - description is taken from the buffer itself.
// - when moved into the job, CPU access is possible only through BufferMapping itself.
unsafe impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped> Mapping
    for BufferMapping<'_, C, T, CMM, MM>
{
    type Client = C;
    type Access = MM;

    fn mapping_id(&self) -> u32 {
        *MM::get_handle_ref(&self.mapping_id)
    }
}
// Safety: mapping has been created on self.channel.
unsafe impl<C: host1x_client::Client, T: BufferType, CMM: MappingMode, MM: Mapped>
    MappedTo<Channel<C>> for BufferMapping<'_, C, T, CMM, MM>
{
    fn is_mapped_to(&self, channel: &Channel<C>) -> bool {
        ptr::eq(self.channel, channel)
    }
}
