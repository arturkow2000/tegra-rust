//! GPU tasks.

use host1x_client::Client;

use crate::cmdseq::CommandSequence;

/// A single unit of GPU work.
///
/// Internally this consists of sequence of one or more Host1x commands.
pub unsafe trait Task: Sized {
    type Client: Client;
    type Buffers;

    fn buffers(self) -> Self::Buffers;
    fn cmdseq(&mut self) -> impl CommandSequence<Self::Client>;
}
