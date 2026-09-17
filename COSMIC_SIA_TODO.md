# Cosmic display: host-to-SIA / Lighting roadmap

## Mission

Turn this MIT-licensed Orbital fork into Cosmic's graphical presentation
service.  The end state is a user-space `displayd` and its client library
running as native SIA32 Cosmic modules on the real
`CpuBoardFpga -> MainboardFpga -> QDX-G CardFpga` LightingSimulation path.

It provides the complete useful Orbital desktop behaviour: windows, focus,
stacking, pointer/keyboard input, decorations, menus/launcher and basic
applications.  It remains a **presentation service**, not a kernel component
and not a hardware-specific graphics API.

The current Rust-to-SIA toolchain and the composed native CPU-board execution
path are not yet sufficient for this to be the immediate executable target.
Until they are, all new portable logic is built and tested on a normal host
(`x86_64-unknown-linux-gnu` is the first supported target).  Host execution
is progress and test evidence for this repository only; it is not evidence of
Cosmic/SIA completion.

## Fixed architecture

```text
applications / NeWS-like runtime
      | Cosmic Present client API
      v
displayd: session, surface, focus, window manager, compositor
      | private QdxGOutput service
      v
qdxg-driver: QDX queues, DMA mapping, presentation completion
      | PLIO
      v
QDX-G: local VRAM, renderer/blitter, scanout, cursor, monitor
```

- The kernel supplies only Tasks, Ports, Signals, Pages, mappings, capability
  transfer and derived interrupt authority.
- `displayd` is the policy owner: placement, z-order, focus, decorations and
  visibility.
- A client owns the writable mapping for its buffers; it cannot map QDX-G
  MMIO, queues, local VRAM or another client buffer.
- Only `qdxg-driver` holds the derived device authority.  It receives
  `InterruptSource -> Signal` delivery and no raw PLIO or machine interrupt
  authority.
- QDX descriptor memory references are `region + offset + length`, never
  physical addresses.  DMA is bounded and capability mediated.
- QDX-G owns local VRAM and continuous scanout.  PLIO is control, bounded
  upload/download and queue traffic -- never a stream of scanout pixels.
- NeWS/PostScript, transforms, paths, text and window policy stay on the CPU
  above the presentation protocol.  QDX-G remains a low-level raster device.

## Protocol target: Cosmic Present v1

Define a versioned, binary, capability-native protocol.  It must not inherit
the Redox scheme/file API as Cosmic's ABI.

### Server-owned objects

- `DisplayService`: creates a session and mints only the capabilities needed
  by that session.
- `Session`: owns the client's surfaces, event endpoint and cleanup.
- `Surface`: a server-tracked presentation object; it is not a memory
  mapping.
- `Toplevel`: optional window-management role on a surface.
- `Seat` and `Output`: event/configuration sources, not direct device
  access.

### Client-owned content

- `Buffer`: shared Pages mapped writable to the client and read-only to
  `displayd`; v1 uses tightly specified `ARGB8888` with stride and bounds.
- `Damage`: bounded rectangle list attached to a commit.
- Clients issue `attach(buffer)`, `damage(rects)`, and atomic
  `commit(serial)`.
- `displayd` issues `configure(size, serial)`, input events,
  `frame_done(serial)`, and `buffer_released(buffer)`.
- A committed buffer remains immutable to that client until
  `buffer_released`.  This is the lifetime rule that permits safe
  double/triple buffering.

Do not add clipboard, drag-and-drop, protocol extensibility, client-side
decorations, GPU buffer sharing, or multi-seat/multi-output policy to v1.

## Work plan

### P0 -- Establish the port boundary

- [x] Add this roadmap to the repository and preserve the upstream Orbital
      license and notices.
- [x] Record the upstream Orbital commit used as the initial source baseline:
      `bf26501c9e9cd40c216408b8f5ba60a6fa476cc5` (upstream Orbital mirror
      state immediately before this roadmap).
- [x] Create a source inventory ([`docs/PORT_BOUNDARY.md`](docs/PORT_BOUNDARY.md)) separating portable code from Redox coupling:
  `src/core.rs`, `window.rs`, `window_order.rs`, widgets and compositor
  algorithms versus `main.rs`, `scheme.rs`, Redox DRM/input/event setup.
- [ ] Do not update upstream just to make the fork look current; keep changes
      reviewable and make later upstream comparisons possible.

**Gate:** the inventory names every direct use of `redox_*`,
`graphics_ipc`, `inputd`, DRM, schemes, filesystem paths and OS event APIs.

### P1 -- Make a portable compositor core (host executable)

- [x] Begin the split with:
  - `cosmic-display-core`: geometry, window/surface state, damage, ordering,
    focus selection and composition planning;
  - `cosmic-present`: wire-safe IDs, formats, events and validation;
  - `cosmic-display-host`: normal-host executable and mock platform;
  - retain an `orbital-compat` layer only where it helps migrate existing
    Orbital clients.
- [x] Make the new portable core use `alloc` and explicit data contracts instead of Redox
      syscalls, schemes, files or device types.  `std` is permitted in the
      host adapter but must not leak into `core` or `cosmic-present`.
- [ ] Introduce narrow traits:
  - `OutputBackend`: acquire backbuffer, present damaged rectangles,
    completion;
  - `InputBackend`: normalized pointer/key/text events;
  - `SessionTransport`: request/event exchange and capability handoff.
- [ ] Keep current Orbital window ordering and rendering behaviour where it
      fits.  Replace scheme-open/read/write/event-poll with explicit session
      operations; do not emulate a Redox scheme inside Cosmic.
- [x] Add deterministic unit tests for surface lifetime, clipped damage,
      layer ordering, focus changes and disconnect cleanup.
- [ ] Add damage coalescing and resize serials after the session/commit API is
      introduced; they cannot be correct as orphaned geometry helpers.

**Gate:** `cargo test --workspace` passes on x86_64 and no portable crate
depends on a Redox-only crate.

The repository's `Host portability` workflow enforces this gate on ordinary
host CI. It intentionally leaves the `redox-compat` binary disabled.

### P2 -- Host composition and client proof

- [ ] Provide a deterministic memory output backend with ARGB8888 buffers.
- [ ] Port or write two tiny clients: a terminal-like opaque surface and an
      alpha/overlap test surface.
- [ ] Test exact pixels for composition, clipped damage, focus/input routing,
      resize acknowledgement, close/disconnect and buffer-release ordering.
- [ ] Add a runnable host demo only after the tests are deterministic.  It may
      use SDL/softbuffer/winit in the host adapter, never in portable core.
- [ ] Keep the host harness able to emit an image/trace on failure for CI
      inspection.

**Gate:** two independently committed client buffers compose correctly,
including an overlapping damage-only frame, with no use-after-release.

### P3 -- Cosmic service adapter contract

- [ ] Specify how System Task launches `displayd.cmod`, grants its initial
      display/input service capabilities, and revokes them on service death.
- [ ] Define the `DisplayService` Port operations and bounded message
      encodings; use shared Pages only for pixels and bulk event batches.
- [ ] Implement a host-side `CosmicTransportMock` exercising the exact
      protocol with capability IDs, Page mapping rights, Signal delivery,
      revocation and task exit.
- [ ] Define input as a separate Cosmic input service.  `displayd` receives
      normalized events and never keyboard-controller MMIO.
- [ ] Define a no-display fallback: applications retain a terminal/serial
      route if `displayd` or QDX-G is absent.

**Gate:** protocol conformance tests prove that clients cannot access another
session's buffer or QDX-G device authority and that task death releases every
surface/buffer deterministically.

### P4 -- QDX-G v1 specification and host model

- [ ] Write `docs/QDX-G-V1.md` in coordination with `rax-plio-qdx`;
      this repository owns the consumer-facing output contract, not PLIO
      hardware internals.
- [ ] Specify v1 as:
  - local ARGB8888 scanout surfaces in QDX-G VRAM;
  - mode setting, double-buffer/page-flip and hardware cursor;
  - one control/submission queue and completion queue;
  - completion notification through Cosmic's public
    `InterruptSource -> Signal` model;
  - bounded upload from displayd composition Pages to QDX-G local surfaces;
  - explicit `PRESENT` completion and reset/fault states.
- [ ] Start with CPU composition into a server-owned backbuffer, then bounded
      upload plus page flip.  Direct client scanout and zero-copy composition
      are later optimizations, not v1 requirements.
- [ ] Provide a pure Rust QDX-G behavioural model for host tests: VRAM,
      page-flip timing, damaged upload bounds, CQ full, reset and stale
      completion cases.  It must be a model/test oracle, not a replacement for
      the future CardFpga acceptance path.
- [ ] Design, but do not require for v1, `EXEC_LIST` acceleration:
      `FILL_RECT`, `COPY_RECT`, `DRAW_SPANS`, `MONO_BLIT`,
      `UPLOAD`, `DOWNLOAD`, `SYNC`, clip/ROP state and cursor/display
      control.  No paths, transforms, fonts or window objects in hardware.

**Gate:** host traces demonstrate that each accepted presentation request
uses only authorized regions, produces exactly one completion or fault, and
never turns PLIO into scanout traffic.

### P5 -- Native Cosmic build readiness

This phase begins only when the relevant toolchain work has landed elsewhere;
do not block P0--P4 on it.

- [ ] Make `cosmic-display-core` and `cosmic-present`
      `#![no_std] + alloc` clean for SIA32, with a documented allocator and
      panic/logging boundary supplied by Cosmic.
- [ ] Build the display client, server and test fixture as native
      `.cmod`/CSM-compatible SIA32 modules through the real Forge/Rust
      pipeline; do not use handwritten SIA, a host JIT or a reference CPU as
      acceptance.
- [ ] Pin and record the exact SIA ABI, relocatable-object, linker/image and
      module-loader revisions used.
- [ ] Replace `CosmicTransportMock` with the actual Port/Page/Signal adapter
      and run the same conformance suite in Cosmic userspace.

**Gate:** compiler-produced SIA code starts `displayd` and two test clients
under Cosmic, with the protocol tests still meaningful.

### P6 -- LightingSimulation and real QDX-G acceptance

- [ ] Add a real QDX-G card through the generic `CardFpga` contract at a
      selectable PLIO slot; no display-specific simulator transport.
- [ ] Connect it only through the composed board epoch:
      registered card input -> bounded card microsteps -> next-epoch
      backplane drive -> one MainboardFPGA evaluation.
- [ ] Load Cosmic and native `displayd` through the ROM/reset/MMU/PLIO boot
      path.  Map client Pages, execute QDX-G queue operations and wait on the
      derived Signal.
- [ ] Capture reproducible diagnostics: commit serial, surface/buffer IDs,
      QDX queue indices, DMA region/offset/length, notification state,
      present state, frame checksum and card/mainboard epoch.
- [ ] Prove:
  1. two native clients create and commit distinct surfaces;
  2. the compositor produces the expected overlapped frame;
  3. QDX-G receives bounded data, flips at presentation boundary and signals
     completion;
  4. reset, malformed descriptors, revoked buffer authority and CQ-full fail
     safely;
  5. at least one hostile/replayable timing corpus remains deterministic.

**Gate:** the same compiler-produced SIA modules execute through the real
CPU-board/Mainboard/CardFpga composition and produce a checked framebuffer
checksum.  A host demo, Rust model or reference interpreter alone does not
satisfy this gate.

### P7 -- Full Orbital desktop compatibility

- [ ] Port retained Orbital decorations, launcher, menu interaction,
      wallpaper/configuration and desktop application integration above
      `Cosmic Present`.
- [ ] Either port `orbclient` behind a Cosmic backend or provide a
      compatibility client crate.  Preserve the old API only as a migration
      aid; Cosmic Present remains authoritative.
- [ ] Add a terminal and diagnostics shell before a file manager or
      application launcher.
- [ ] Add text/font rasterization and the optional NeWS-like runtime above
      `displayd`; keep these unprivileged and capability-scoped.
- [ ] Add QDX-G EXEC_LIST acceleration only after CPU composition has passed
      the native Lighting proof.  Differential-test accelerated output
      against the software compositor.
- [ ] Consider multiple outputs, remote/GNet presentation, clipboard and
      direct scanout as separate proposals after v1 is stable.

**Gate:** normal desktop interaction works on real SIA/Lighting hardware with
the compositor, clients and QDX-G all separated by the intended capability
boundaries.

## Explicit non-goals until after the native proof

- A Wayland/X11 server or Linux ABI compatibility layer.
- Kernel window management, graphics policy or widget toolkits.
- Direct application MMIO/QDX queue/VRAM access.
- A general GPU, 3D API, shader stack or WebGPU/Vulkan implementation.
- Continuous framebuffer DMA over PLIO.
- Treating host execution, fake SIA execution or manually encoded instructions
  as evidence of the native milestone.

## Immediate next task

Start **P0/P1**: write the source inventory, create the portable workspace
split, and add the memory-output compositor tests.  This is valuable now on
x86_64 and is deliberately independent of unfinished SIA compiler work.
