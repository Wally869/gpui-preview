# Frame Capture

gpui-preview includes a forked version of gpui (0.2.2) with a `capture_frame` API that reads rendered pixels back from the GPU. This enables screenshot-based testing, pixel diffing, and automated visual regression workflows without relying on OS-level screen capture.

## How it works

GPUI renders UI through platform-specific GPU backends:

| Platform | Backend | Render Target |
|----------|---------|---------------|
| Windows | DirectX 11 | `ID3D11Texture2D` swap chain |
| macOS | Metal | `CAMetalLayer` drawable |
| Linux | Blade (Vulkan/GL) | `gpu::Surface` frame texture |

Each frame, GPUI builds a `Scene` (a list of paint primitives: quads, shadows, text sprites, paths, etc.), sends it to the GPU renderer, and presents the result to the window.

The fork adds a readback step **after rendering but before/during presentation** on each platform:

### Windows (DirectX 11)

1. Creates a **staging texture** (`D3D11_USAGE_STAGING` with `CPU_ACCESS_READ`)
2. Calls `CopyResource` from the render target to the staging texture
3. Maps the staging texture to CPU memory and copies the pixel data out

### macOS (Metal)

1. Maintains a persistent **capture texture** (`MTLStorageMode::Managed`)
2. After rendering, blits the drawable texture to the capture texture via `MTLBlitCommandEncoder`
3. Calls `synchronize_resource` to flush to CPU, then reads with `get_bytes`

### Linux (Blade/Vulkan)

1. Maintains a persistent **shared-memory buffer** (`gpu::Memory::Shared`)
2. After rendering, issues a `copy_texture_to_buffer` transfer command
3. After GPU submission and sync, reads directly from the buffer's mapped pointer

## API

```rust
// On Window:
pub fn capture_frame(&self) -> Option<(u32, u32, Vec<u8>)>
```

Returns `Some((width, height, pixels))` where `pixels` is BGRA8 data (4 bytes per pixel, row-major order), or `None` if no frame has been rendered yet.

## Usage

```rust
window_handle.update(cx, |_view, window, _cx| {
    if let Some((width, height, bgra_data)) = window.capture_frame() {
        // Convert BGRA to RGBA if needed (e.g. for the `image` crate)
        let mut rgba = bgra_data;
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        image::save_buffer("screenshot.png", &rgba, width, height, image::ColorType::Rgba8)
            .expect("failed to save");
    }
});
```

## The fork

The fork lives in `vendor/gpui/` and is wired in via a Cargo patch in the workspace root:

```toml
[patch.crates-io]
gpui = { path = "vendor/gpui" }
```

This means **all** crates that depend on `gpui` (including `gpui-component`) automatically use the patched version. No other forks are needed.

### Files changed from upstream gpui 0.2.2

- `src/platform.rs` — added `capture_frame()` default method on `PlatformWindow` trait
- `src/window.rs` — added public `Window::capture_frame()`
- `src/platform/windows/directx_renderer.rs` — D3D11 staging texture readback
- `src/platform/windows/window.rs` — wired `capture_frame` through
- `src/platform/mac/metal_renderer.rs` — Metal blit + managed texture readback
- `src/platform/mac/window.rs` — wired `capture_frame` through
- `src/platform/blade/blade_renderer.rs` — Blade transfer + shared buffer readback
- `src/platform/linux/wayland/window.rs` — wired `capture_frame` through
- `src/platform/linux/x11/window.rs` — wired `capture_frame` through

## Limitations

- **Not truly headless**: The GPU needs a window surface to render. Windows briefly appear on screen during capture. Setting `show: false` causes the GPU to skip rendering (blank output). Off-screen positioning causes Windows to enforce a large minimum size.
- **Size matching**: The capture size matches the window size, not the component's natural size. If the window is larger than the component, you get empty space. Size your capture to match the component dimensions.
- **DPI scaling**: Captured pixels are in physical resolution. A 400x300 window at 150% DPI produces a 600x450 image.

## Intended use: pixel diffing

The end goal is to compare GPUI-rendered components against HTML/CSS reference implementations:

1. Render a component via gpui-preview, capture to PNG
2. Render the same component in a browser (HTML/CSS), screenshot to PNG
3. Pixel-diff the two images to validate visual parity

This enables teams migrating between web and native UI to verify that their GPUI components match their design system's HTML/CSS reference.
