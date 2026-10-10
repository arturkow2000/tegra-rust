# WIP driver for Tegra GPUs

This is a work-in-progress GPU driver for old Tegra devices (currently Tegra
20 and GR2D only).

## Project scope

The project aims to provide everything that is required for working and
usable GPU - that is, running full graphical session on latest Linux with
GPU acceleration wherever needed (e.g. any modern web browser).

Currently I'm focusing on GR2D (fixed-function 2D accelerator) and I plan to
integrate that (at minimum) with Wayland compositor of choice - that probably
would mean wlroots as it's the most complete implementation (high-level
driver APIs will be exposed over FFI).

### On 3d acceleration

Currently it's very early to think about this. I want first to create strong
foundation, before proceeding with anything other than GR2D.

Currently I consider getting proprietary Nvidia 3d working by translating
old nvhost calls to DRM and using custom layers to integrate old driver with
wayland (to serve both as temporary solution until open-source 3d driver is
ready, and as testbed).

## Relationship with [grate](https://github.com/grate-driver/grate)

I started this project as rewrite/continuation of grate driver, which at the
time has been dead for ~2 years (resumed on July 2026).

Currently, Grate strongly depends on X11 which is deprecated and being phased
out by Linux distributions desktop environments. This project aims to bring
modern future-proof drivers, and hopefully will become the official
implementation.

## Features

Non-exhausive list of supported and planned features

- [x] Linux DRM backend
- [x] Command stream submission
- [ ] Type-safe APIs for building execution graphs
- [x] Basic GR2D APIs (currently clear and fill only)
- [ ] Full GR2D APIs
- [ ] FFI for GR2D driver
- [ ] GR2D renderer for wlroots

## Running

**WIP**

Quick note:

You can use provided devcontainer to cross compile for Tegra 20 device. Open in
devcontainer then do:

```
cargo -Zjson-target-spec -Zbuild-std build --target=tegra20-unknown-linux-gnueabihf.json
```

Use latest mainline kernel. `basic-test` does offscreen rendering, you will need
[tracy](https://github.com/wolfpld/tracy) on the host to see anything.
Alternatively you may use `demo` branch to see green rectangle on the screen
(before app crashes).
