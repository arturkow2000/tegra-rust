# CLAUDE.md

WIP userspace GPU driver for old NVIDIA Tegra SoCs, written in Rust. Currently
targets **Tegra 20** and only the **GR2D** fixed-function 2D engine, talking to the
mainline Linux `tegra` DRM driver. Long-term goal: a GR2D renderer for wlroots
exposed over FFI. See [README.md](README.md) for project scope and roadmap.

## Build, test, run

You are already running inside the Nix-based devcontainer ([flake.nix](flake.nix),
[.devcontainer/](.devcontainer/)). It provides the pinned nightly toolchain, the clang cross
toolchains, and the env vars they need, so no setup is required.

- Build the project (Tegra 20 target spec: no NEON, VFPv3-D16, Thumb-2):
  ```
  cargo build -Zbuild-std -Zjson-target-spec --target=tegra20-unknown-linux-gnueabihf.json
  ```
  A `musleabihf` spec also exists (static). Use the JSON specs instead of plain
  `armv7-*`: generic armv7 code may use NEON and crash on T20.
- `cargo run` with a tegra20/armv7 target uses [.vscode/remote-run.sh](.vscode/remote-run.sh)
  as the runner (set in [.cargo/config.toml](.cargo/config.toml)). It uploads the binary to
  `root@192.168.0.163` over SSH and runs it there with `RUST_LOG=debug`. This needs real hardware.
- Host tests work for the `no_std`/pure crates:
  ```
  cargo test -p host1x-core -p colorspace
  ```
  Most coverage in `host1x-core` is **doctests**, including `compile_fail` ones that check
  that invalid commands are rejected at compile time (E0080 const-eval asserts). Keep
  them passing when you change command encoding. `host1x-drm` and `basic-test` need the
  Tegra DRM device, so they can only be exercised on hardware.
- `.cargo/config.toml` sets `--cfg tracing_unstable` (needed for `valuable` fields in tracing).
- rust-analyzer is configured for `armv7-unknown-linux-gnueabihf` with `-Zbuild-std`
  ([.vscode/settings.json](.vscode/settings.json)).
- `external/generativity` is a submodule with a fork of `generativity` (branch `super-let`). It carries
  upstream PR #20, which adds the `guard!()` expression macro, and marks `guard!` with
  `allow_internal_unstable(super_let)` so callers need no feature gate. It also removes `make_guard!`, so
  brands are created with `guard!()` only. The fork requires nightly. It is excluded from the workspace
  and used as a path dependency. Run its tests with `cargo test` inside the directory, and re-run them
  after nightly bumps: brand soundness depends on `super let` semantics.
- `external/LinuxTracepoints-Rust` and `external/perf-event` are gitlinks without `.gitmodules`
  entries, so they can't be checked out. They aren't used yet.

## Crate layout (bottom-up)

| Crate | `no_std` | Purpose |
|---|---|---|
| [host1x-client](crates/host1x-client/) | yes | `Client` trait: a Host1x hardware block identified by a 10-bit `CLASS` ID. Used as a type-level tag everywhere. |
| [colorspace](crates/colorspace/) | yes | Pixel formats (`Format`, `Encoding`, `Layout`, e.g. `Rgba8888`, `Bgra8888`), typed `Color<E>`, get/set pixel helpers, drm-fourcc mapping. Layout-independent encodings live in `encoding` (aliases `encoding::{Rgb8, Rgba8, Rgb565}`). Formats and the existing encoding items are re-exported at the root, the aliases are not, so they don't clash with format names. |
| [host1x-buffer](crates/host1x-buffer/) | yes | Buffer traits: `Buffer`, `PixelBuffer`, and the per-client `MappedBuffer<C>` / `ReadableBuffer<C>` / `WritableBuffer<C>`. |
| [host1x-reg](crates/host1x-reg/) | yes | `impl_registers!` macro on top of `tock-registers`; `Register<C>` / `RegisterValue<C>` traits; `reloc::Relocatable` for registers that hold buffer addresses; `class` = Host1x class methods (`INCR_SYNCPT`). |
| [gr2d-reg](crates/gr2d-reg/) | yes | GR2D register definitions (class `0x51`) and `gr2d_reg::Client`. |
| [host1x-core](crates/host1x-core/) | yes (`std` feature) | Type-safe command stream building (`cmd`: `SetClass`, `Mask`, `WriteIncr`, `WriteNoIncr`), `cmdseq::CommandSequence`, the `task::Task` trait, and backend-agnostic `queue::{Queue, Fence, PendingTask}` traits. |
| [gr2d](crates/gr2d/) | yes | High-level GR2D operations as `Task`s: `clear`, `clear_with_color`, `fill_rect` (`fill::Fill`). `fill_rect` takes any `fill::FillColor`: a color with the buffer's RGB channels and any alpha. `NoAlpha` or `0xff` alpha fills, other alpha blends with fixed alpha (`ALPTYPE::FIX`). `clear*` always write the raw color. `Gr2dFormat` maps colorspace formats to SBFMT0 values. |
| [host1x-drm](crates/host1x-drm/) | no | Linux DRM backend: `device::TegraDRM`, GEM buffers (`uapi::v1::buffer`), Host1x channels / mappings / syncpoints / submit queue (`uapi::v2`), DRM syncobj fences (`fence`). Raw ioctl definitions are in `uapi/*/ioctl.rs`. |
| [instrumentation](crates/instrumentation/) | no | tracing + Tracy setup (`init()`); `TracingContext::frame()` uploads a pixel buffer to Tracy as a frame image. |
| [basic-test](crates/basic-test/) | no (bin) | End-to-end smoke test: opens `/dev/dri/card1`, clears a 256×256 BGRA buffer and fills a rect with GR2D, then sends the result to Tracy. |

## Core design: how a GPU job flows

1. **Registers** are declared with `impl_registers![Client, pub NAME OFFSET(..) RELOCATABLE(bool) [fields]]`.
   Each register becomes a module (`gr2d_reg::CTRLMAIN::TURBOFILL::ENABLE`, `DSTSIZE::DSTWIDTH.val(w)`,
   `X::raw(..)`, `X::zero()`). `RELOCATABLE(true)` registers have no fields. Instead you build them from a
   branded buffer handle with `read(&buf)` / `write(&mut buf)` / `new(&mut buf)`. The returned
   `Relocatable<'id, ..>` carries the buffer's brand, not a borrow, so the handle can be moved on.
2. **Commands** (`host1x_core::cmd`) take tuples of register values. Register ordering, mask width
   (16 bits for `Mask`, 6 for `SetClass`), contiguity for `WriteIncr`, and word/reloc counts are all
   checked **at compile time** through const generics and `assert!` in consts. The const generics are
   `<N, N_RELOCS, Client, Args>`, and `_` usually works for the inferred parts.
3. **Command sequences**: tuples of `CommandSequence`s are themselves `CommandSequence`s (implemented
   up to 15 elements; `recursion_limit = "256"` is needed for this). They can be written into
   `&mut [u32]` or `&mut [MaybeUninit<u32>]` and expose their relocations.
4. **Jobs** ([job.rs](crates/host1x-core/src/job.rs)): the API is functional, so the whole
   task sequence is built in one call:
   `let job = job!(buffers, (|buf| task1(buf), |buf| task2(buf)));`. It is equivalent to
   `let g = guard!(); let job = Job::new(g, buffers, pipeline);`, and creates the brand with
   generativity's `guard!()` held in a `super let`. Callers need no feature gate, because both macros
   use `allow_internal_unstable(super_let)`. host1x-core enables `allow_internal_unstable` for this.
   This moves the real buffers (anything implementing `Import`, including non-empty tuples; `()` is
   deliberately not `Import`, so a job always owns at least one buffer) into the job, and
   passes `Handle<'id, C, Layout, Access>`s to the pipeline. Handles are branded with a unique invariant
   lifetime from `generativity`. Only handles implement `MappedBuffer` / `ReadableBuffer` /
   `WritableBuffer`, so the CPU can't access a buffer while the GPU uses it. These are `unsafe`
   traits: tasks trust their mapping ID and layout to program the GPU. Handles are linear
   (not `Copy` / `Clone`).
5. **Pipelines** (sealed `Pipeline` trait): a pipeline is a closure `FnOnce(In) -> impl Task<'id>`, a
   tuple of pipelines run in order (they can be nested, up to 16 elements per tuple). There is no
   empty pipeline (`()` is deliberately not a `Pipeline` nor a `CommandSequence`), so a job always
   runs at least one task and command sequences are never empty. Each stage receives the previous
   stage's `Task::Output`. `task.with(extra)` passes handles the task doesn't use
   on to the next stage. Since closures can't return borrows of their arguments, tasks that read a
   buffer should take it by value and return it in `Output`.
   `Task::split(self) -> (impl CommandSequence, Output)` returns the commands plus the handles to
   pass on. Pipelines nest sequences as `(first, rest)` pairs, so the size of the **whole** stream
   (`N_WORDS` / `N_RELOCS`) is known at compile time. Keep it that way, and don't replace this with
   runtime recording.
6. **Submission** (`host1x_core::queue::Queue`, implemented by `host1x_drm::uapi::v2::queue::Queue`):
   `queue.submit(&mut fence, job).await?` returns a `Pending` future, and awaiting that gives back
   the buffers. Buffers can only be mapped to Host1x channels (the abstraction the kernel driver
   exposes), represented by the `queue::Channel` trait. `Queue::Channel` is the backend's channel
   type, and `Job::into_parts(channel)` asserts that every buffer is mapped to it.
   The queue appends an `INCR_SYNCPT` (OP_DONE) and signals a DRM syncobj. The syncobj wait runs on
   `tokio::task::spawn_blocking`, so a tokio runtime is required. No barriers are needed between
   tasks, because the hardware orders them through syncpt conditions. Dropping `Pending` early drops
   the buffers, which is sound.

### host1x-drm typestates
- Mapping modes (both CPU and Host1x) are the access markers from `host1x_buffer::access`
  (`ReadOnly`, `WriteOnly`, `ReadWrite`), plus host1x-drm's `Unmapped` for "no CPU mapping".
  [mapping.rs](crates/host1x-drm/src/mapping.rs) implements the extension traits `MappingMode` (handle
  storage, mmap) and `Mapped: Access` (Host1x mapping) for them. Readability and writability use
  `host1x_buffer::access::{Readable, Writable}`, so `WriteOnly` mappings can't be read from the CPU.
- `GenericBuffer<MM, T>`: `MM` is the CPU mapping mode, and `T: BufferType` (currently
  `buffer::PixelBuffer<F>`). `Buffer<T>` is the unmapped alias. Convert with `mmap_ro()` / `mmap_rw()`.
- `Channel<C>::map_buffer(buffer)` takes the buffer **by value** and returns
  `BufferMapping<'_, C, T, CMM, MM>`. It implements `Import` (currently only for pixel buffers) and
  `MappedTo<Channel<C>>` instead of the GPU traits, derefs to the bytes when CPU-readable, and
  `unmap()` returns the buffer.
- `TegraDRM::try_from_fd` checks that the driver name is `"tegra"` and that syncobj is supported.
  Much of the crate's safety argument depends on this check.
- GEM and syncobj handles owned by the crate's objects live in the DRM fd's namespace, and
  `drm`/`drm-ffi` have safe APIs to close, signal or export them. So `TegraDRM` must not expose
  its fd: it implements neither `AsFd` nor the `drm` device traits (use private `as_fd()` or unsafe
  `drm()`; `AsRawFd` is fine, raw fds need `unsafe` to be used), `try_from_fd` is `unsafe`, and so
  are raw handle getters like `Syncobj::handle()`.

## Conventions

- Nightly-only. You may bump the pinned toolchain in [rust-toolchain.toml](rust-toolchain.toml)
  when needed.
- Unstable features may be introduced when they bring real benefits. Never introduce
  **incomplete** features (those that trigger the `incomplete_features` lint, e.g.
  `generic_const_exprs`) without asking first. They can cause UB in otherwise valid code, so avoid them.
- Edition 2024, resolver 3. Shared deps live in `[workspace.dependencies]`; internal crates use
  `name.path = "../name"`.
- Errors: `snafu` (`no_std` in core crates), `io::Result` in the DRM backend.
- Prefer `#[derive(..)]` over hand-written impls whenever a derive can generate them (`Debug`,
  `Clone`, `Copy`, `PartialEq`, ...). Write impls by hand only when a derive can't express them,
  e.g. when it would add unwanted bounds on generic parameters or must skip fields.
- Sealed and private traits are a common pattern (`mod p { pub trait FooPriv }` + `pub trait Foo: p::FooPriv {}`,
  or `mod sealed`). This keeps safety-critical impls inside the crate.
- `unsafe trait`s (`CommandSequence`, `Task`, `MaskArgs`, `RegisterValue`, ...) carry documented
  `# Safety` invariants, mostly "iterator yields exactly N items". Uphold them in new impls.
- Logging uses `tracing` with `#[macro_use] extern crate tracing;` in std crates, plus `#[instrument]`
  and spans on drop. `Drop` impls `unwrap()` ioctl results only under `debug_assertions`.
- Commit messages are prefixed with the crate or file: `host1x-drm: ...`, `README.md: ...`.

## Known limitations / gotchas

- Only GR2D, and only BGRA8888 and RGBA8888 in `Gr2dFormat`. Only fill and clear are implemented.
- `Submit` does the submit ioctl synchronously inside `poll` (most types are `!Send`).
- `SyncobjWait`'s blocking job isn't scoped, so it could outlive the syncobj handle (FIXME in [fence.rs](crates/host1x-drm/src/fence.rs)).
- Relocation `shift` and `buf_offset` exist, but the safe API always uses 0.
- tracy-sys's C++ needs explicit T20 `CFLAGS`/`CXXFLAGS` (set by the flake), or it builds with NEON and crashes.
