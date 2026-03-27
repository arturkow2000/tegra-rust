//! Host1x command push buffer implementation.

use core::{marker::PhantomData, mem::MaybeUninit, slice};

use host1x_client::Client;
use host1x_reg::reloc::Relocation;

use crate::cmdseq::{CommandSequence, ValidRawCommandArray};

/// Statically sized push buffer.
pub struct PushBuffer<'a, C: Client> {
    cmd_storage: &'a mut [MaybeUninit<u32>],
    reloc_storage: &'a mut [MaybeUninit<Relocation>],
    n_cmd_words: usize,
    n_relocs: usize,
    _client: PhantomData<C>,
}

impl<'a, CL: Client> PushBuffer<'a, CL> {
    /// Create a new empty [`PushBuffer`].
    pub fn new(
        cmd_storage: &'a mut [MaybeUninit<u32>],
        reloc_storage: &'a mut [MaybeUninit<Relocation>],
    ) -> Self {
        Self {
            cmd_storage,
            reloc_storage,
            n_cmd_words: 0,
            n_relocs: 0,
            _client: PhantomData,
        }
    }

    #[inline]
    /// Return the total number of host1x command words the buffer can hold.
    pub fn cmdbuf_capacity(&self) -> usize {
        self.n_cmd_words
    }

    #[inline]
    /// Return the total number of relocations the buffer can hold.
    pub fn reloc_capacity(&self) -> usize {
        self.n_relocs
    }

    #[inline]
    /// Return raw command buffer.
    ///
    /// Note that Host1x commands may take 32-bit words as argument, so commands
    /// are variable-length.
    pub fn cmdbuf(&self) -> ValidRawCommandArray<'_> {
        // Safety:
        // - n_cmds is never larger than cmd_storage.len()
        // - cmd_storage[..n_cmds] have already been initialized
        unsafe {
            ValidRawCommandArray::assume_valid(slice::from_raw_parts(
                self.cmd_storage.as_ptr() as *const u32,
                self.n_cmd_words,
            ))
        }
    }
    #[inline]
    /// Return raw relocation metadata.
    pub fn relocs(&self) -> &[Relocation] {
        // Safety:
        // - n_relocs is never larger than reloc_storage.len()
        // - reloc_storage[..n_relocs] has already been initialized
        unsafe {
            slice::from_raw_parts(
                self.reloc_storage.as_ptr() as *const Relocation,
                self.n_relocs,
            )
        }
    }

    /// Clears the buffer, removing entire command stream and all relocation metadata.
    pub fn clear(&mut self) {
        self.n_cmd_words = 0;
        self.n_relocs = 0;
    }

    /// Push commands into the buffer.
    ///
    /// # Panics
    /// Panics if push would overflow the buffer.
    pub fn push<C: CommandSequence<CL>>(&mut self, seq: C) {
        let relocs = seq.relocs();
        if relocs.len() + self.n_relocs > self.reloc_storage.len() {
            panic!("push would overflow relocation buffer");
        }
        seq.try_write_to_slice(&mut self.cmd_storage[self.n_cmd_words..])
            .expect("push would overflow command buffer");
        self.n_cmd_words += C::N_WORDS;

        for reloc in relocs {
            self.reloc_storage[self.n_relocs] = MaybeUninit::new(reloc);
            self.n_relocs += 1;
        }
    }
}
