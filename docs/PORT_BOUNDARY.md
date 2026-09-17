# Orbital to Cosmic: P0 port boundary inventory

This inventory is intentionally factual rather than aspirational.  The source
tree remains the upstream Orbital implementation; the new workspace crates
are the first Cosmic-owned, host-testable boundary.

## Source classification

| Source | Current role | Classification | Cosmic direction |
| --- | --- | --- | --- |
| `src/main.rs` | Redox daemon startup, `VT`, logger, `inputd`, login command | Redox-only | Replace with a Cosmic module/service entry point later. |
| `src/core.rs` | Scheme registration, syscall transport, event loop, input consumer, display opening | Mostly Redox-only | Split its policy-independent properties out gradually; replace scheme/event APIs with Ports and Signals. |
| `src/scheme.rs` | Orbital file/scheme ABI, input dispatch, decorations, focus, tiling, clipboard, audio paths | Mixed | Retain behaviour only after moving it behind Cosmic Present sessions; do not port the scheme ABI. |
| `src/compositor/display.rs` | DRM modesetting, dumb buffers, scanout and cursor | Redox/Linux DRM-only | Replace with a private QDX-G output adapter; QDX-G owns local scanout. |
| `src/compositor/mod.rs` | Damage scheduling, cursor policy, multi-display redraw coordination | Mixed | Damage policy is portable; DRM display/cursor calls are adapter-only. |
| `src/window.rs` | Window geometry, flags, damage/event queues, decoration/text drawing, Orbclient images | Mixed | Re-home geometry/surface state first; leave images, fonts, decorations and Orbclient events in compatibility code. |
| `src/window_order.rs` | Focus ordering and `Back`/`Normal`/`Front` bands | Portable | First extracted as `cosmic-display-core::SurfaceStack`. |
| `src/config.rs` | TOML/filesystem config and Orbclient colors | Host/compatibility | Keep out of portable core; later make configuration a service-level concern. |
| `src/widget/fps.rs` | Host time, font and image OSD | Host/compatibility | Defer until the base presentation service works. |
| `src/widget/shortcuts.rs` | Orbclient/font shortcut OSD | Host/compatibility | Defer; shortcut policy belongs above the core. |

## Direct dependency classification

| Dependency | Coupling | Port treatment |
| --- | --- | --- |
| `redox_syscall`, `redox_scheme`, `libredox`, `redox_event`, `redox-log` | Redox syscall, scheme, event, logging APIs | Compatibility implementation only. |
| `drm`, `graphics-ipc` | DRM buffers/modesetting and graphics IPC | Replace with private QDX-G adapter; never expose to clients. |
| `inputd` | Keyboard/pointer device consumer | Replace with an input service feeding normalized events. |
| `orbclient`, `orbfont` | Existing client ABI, images, rectangles, events, font rasterization | Keep only in Orbital compatibility/decorations; do not make them Cosmic Present ABI. |
| `serde`, `toml`, `log`, `thiserror` | Host configuration and diagnostics | Host/compatibility unless a portable use is justified later. |

## Initial workspace split

- `cosmic-present` is `#![no_std]` protocol metadata: IDs, ARGB8888 buffer
  descriptors, bounds and rectangle clipping.
- `cosmic-display-core` is `#![no_std] + alloc` surface/focus/order/damage
  state. It has no OS, device, renderer, font, filesystem or event dependency.
- `cosmic-display-host` is a deterministic recorder used for x86_64 tests. It
  is neither a GUI demo nor a QDX-G substitute.
- The root `orbital` package retains the upstream compatibility source, but
  its sole binary is behind the explicit `redox-compat` feature. This prevents
  Cargo from building its Redox-only daemon in the portable workspace. A later
  `orbital-compat` adapter can opt in to that source without making Redox
  dependencies part of the host gate.

## P0 completion condition

Every current Redox-specific dependency and source seam above is named.  New
portable code must enter a workspace crate, not the inherited scheme or DRM
path.
