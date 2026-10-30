// Mock buffer, channel and task shared by tests and doctests.
//
// Not part of the crate's API: unit tests use it as a `cfg(test)` module, and
// doctests pull it in with `include!`. Paths go through `host1x_core` so they
// resolve the same way in both cases.

use gr2d_reg::{Client as Gr2d, DSTBA};
use host1x_buffer::{Buffer, Layout, LinearDesc, WritableBuffer, access::ReadWrite};
use host1x_core::{
    cmd::Mask,
    cmdseq::CommandSequence,
    job::{MappedTo, Mapping},
    queue::Channel,
    task::Task,
};

/// Buffer with given mapping ID.
pub struct MockBuffer(pub u32);
impl Layout for MockBuffer {
    fn size(&self) -> u32 {
        0
    }
}
impl Buffer for MockBuffer {
    type Desc = LinearDesc;

    fn desc(&self) -> LinearDesc {
        LinearDesc::new(0)
    }
}
unsafe impl Mapping for MockBuffer {
    type Client = Gr2d;
    type Access = ReadWrite;

    fn mapping_id(&self) -> u32 {
        self.0
    }
}

pub struct MockChannel;
impl Channel for MockChannel {
    type Client = Gr2d;
}

unsafe impl MappedTo<MockChannel> for MockBuffer {
    fn is_mapped_to(&self, _channel: &MockChannel) -> bool {
        true
    }
}

pub struct Write<B>(pub B);
unsafe impl<'id, B: WritableBuffer<'id, Gr2d>> Task<'id> for Write<B> {
    type Client = Gr2d;
    type Output = B;

    fn split(self) -> (impl CommandSequence<Gr2d>, B) {
        let mut buf = self.0;
        (Mask::<1, 1, Gr2d, _>::new((DSTBA::write(&mut buf),)), buf)
    }
}
