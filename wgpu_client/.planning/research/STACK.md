# Stack Research: Visual Regression Testing for GPU-Rendered IDE UI

**Domain:** Visual regression testing for wgpu-based applications
**Researched:** 2026-01-28
**Confidence:** HIGH

## Recommended Stack

### Core Testing Infrastructure

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| insta | 1.46.1 | Snapshot testing framework | Industry-standard snapshot testing for Rust. Handles snapshot storage, diffing, and review workflow. Excellent CI integration with auto-detection. Battle-tested in major Rust projects. |
| cargo-insta | 1.46.1 | CLI tool for snapshot review | Essential companion to insta. Provides interactive snapshot approval, inline snapshots, and batch operations. VSCode extension available. |
| image | 0.25.9 | Image loading/saving | De facto standard for image I/O in Rust. Supports PNG (lossless), handles color spaces correctly, no external C dependencies. Required for screenshot persistence. |

### Image Comparison Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| image-compare | 0.5.0 | Structural similarity and RMS comparison | Primary choice for visual regression. Supports SSIM (perceptual similarity), RMS (pixel difference), hybrid RGBA comparison. Pure Rust, fast, good defaults. |
| dssim | 3.4.0 | Advanced perceptual similarity (multiscale SSIM) | When SSIM isn't strict enough. Simulates human perception better with L*a*b* color space and multiple resolution scales. CLI tool available for manual inspection. AGPL licensed (commercial license available). |

### Screenshot Capture (wgpu-specific)

| Approach | Components | Purpose | Implementation Notes |
|----------|-----------|---------|---------------------|
| Render-to-texture + buffer readback | wgpu::Texture, wgpu::Buffer, copy_texture_to_buffer | Capture GPU-rendered frames | Create texture with RENDER_ATTACHMENT + COPY_SRC usage. Create buffer with COPY_DST + MAP_READ. Use encoder.copy_texture_to_buffer() then buffer.map_async() to read pixels. See Learn Wgpu windowless tutorial. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-insta VSCode extension | Snapshot review in editor | Syntax highlighting for .snap files, inline approval, jump-to-definition. Optional but highly recommended for productivity. |
| pollster | Block on async operations in tests | Minimal async runtime for blocking on buffer mapping in tests. Alternative: use #[tokio::test] if already using tokio. |

## Installation

```toml
[dev-dependencies]
# Snapshot testing framework
insta = { version = "1.46.1", features = ["yaml"] }

# Image comparison (choose based on needs)
image-compare = "0.5.0"  # Recommended: good balance of speed/accuracy
# OR
dssim-core = { version = "3.4.0", default-features = false }  # Advanced: better perceptual matching

# Image I/O
image = { version = "0.25", features = ["png"] }

# Async support for wgpu buffer mapping
pollster = "0.4"
```

```bash
# Install cargo-insta CLI tool
cargo install cargo-insta --version 1.46.1

# Optional: Install VSCode extension
# Search for "Insta Snapshots" in VSCode marketplace
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| insta | Manual assert_eq! with image bytes | Never. Manual comparison is brittle, produces unreadable diffs, and has no review workflow. |
| image-compare | dssim | When you need the highest perceptual accuracy and can tolerate AGPL licensing. dssim's multiscale approach catches subtle differences that SSIM misses. |
| Render-to-texture | Screen capture APIs (e.g., winit screenshot) | Never for testing. Screen capture requires visible window, can't run headless in CI, and captures OS chrome/artifacts. Always render to texture for tests. |
| insta | Jest-image-snapshot, BackstopJS | When your project is JavaScript/Node.js. These are excellent tools but not available for Rust. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Pixel-perfect comparison (memcmp) | GPU drivers produce non-deterministic output across hardware. Even same GPU with different drivers can vary by 1-2 RGB values per pixel. CI will be flaky. | Use SSIM-based comparison with threshold (e.g., 0.01 for image-compare, 0.001 for dssim). |
| Exact floating-point comparisons | Rounding differences across GPUs/drivers cause failures. | Use epsilon comparisons or perceptual metrics. |
| Windows-only screenshot tools | Won't run in Linux CI environments (GitHub Actions). | Use wgpu's cross-platform render-to-texture. |
| xray crate (from blog post) | Unmaintained, last update 2020. Good concepts but outdated dependencies. | Implement similar pattern with modern image + insta stack. |

## Stack Patterns by Use Case

**For IDE UI with text rendering (your use case):**
- Use image-compare with SSIM algorithm (not RMS)
- Set threshold around 0.005-0.01 (text is high-frequency, needs tolerance)
- Consider glyphon rendering variations: font hinting differs across platforms
- Capture at fixed DPI (96 or 144) to avoid scaling artifacts
- Use PNG format (lossless) for snapshots

**For geometric rendering (shapes, lines):**
- Can use stricter threshold (0.001-0.005)
- RMS comparison may be sufficient
- Watch for anti-aliasing differences

**For complex scenes with gradients:**
- Use dssim for better perceptual matching
- Set threshold based on human-visible differences (dssim 0.0005 ≈ "just noticeable")

**For CI integration:**
- Let insta auto-detect CI environment (sets INSTA_UPDATE=no)
- Use `cargo insta test --check` to fail on new/changed snapshots
- Upload .snap.new files as GitHub Actions artifacts for review
- Use `INSTA_UPDATE=always` in local dev to auto-accept changes

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| insta@1.46.1 | cargo-insta@1.46.1 | Must match exactly. Cargo-insta reads/writes snapshot format. |
| image-compare@0.5.0 | image@0.25.x | image-compare uses image crate internally. Keep image up to date for format support. |
| wgpu@0.23.x | pollster@0.4.x | pollster is runtime-agnostic, works with any wgpu version. |
| insta@1.46.1 | serde@1.x | insta requires serde for YAML/JSON snapshots. Most projects already have this. |

## Screenshot Capture Workflow (wgpu-specific)

Based on Learn Wgpu windowless tutorial and community patterns:

```rust
// 1. Create texture with correct usage flags
let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("Test Render Texture"),
    size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
         | wgpu::TextureUsages::COPY_SRC,  // Enable copying to buffer
    view_formats: &[],
});

// 2. Render to texture (your existing render code)
// ... render pass code ...

// 3. Create output buffer for readback
let bytes_per_row = 4 * width; // 4 bytes per RGBA pixel
let padded_bytes_per_row = ((bytes_per_row + 255) / 256) * 256; // 256-byte alignment
let buffer_size = padded_bytes_per_row * height;

let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("Screenshot Buffer"),
    size: buffer_size as u64,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
});

// 4. Copy texture to buffer
let mut encoder = device.create_command_encoder(&Default::default());
encoder.copy_texture_to_buffer(
    wgpu::TexelCopyTextureInfo {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    },
    wgpu::TexelCopyBufferInfo {
        buffer: &output_buffer,
        layout: wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(padded_bytes_per_row),
            rows_per_image: Some(height),
        },
    },
    wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
);
queue.submit(Some(encoder.finish()));

// 5. Map buffer and read pixels
let buffer_slice = output_buffer.slice(..);
let (tx, rx) = std::sync::mpsc::channel();
buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    tx.send(result).unwrap();
});
device.poll(wgpu::Maintain::Wait);
rx.recv().unwrap().unwrap();

// 6. Extract pixel data
let data = buffer_slice.get_mapped_range();
// Note: data has padding! Must unpad to get actual image bytes
let mut pixels = Vec::with_capacity((width * height * 4) as usize);
for row in 0..height {
    let start = (row * padded_bytes_per_row) as usize;
    let end = start + (width * 4) as usize;
    pixels.extend_from_slice(&data[start..end]);
}
drop(data); // Drop before unmapping
output_buffer.unmap();

// 7. Create image and save
let img = image::RgbaImage::from_raw(width, height, pixels).unwrap();
img.save("screenshot.png").unwrap();
```

**Critical gotchas:**
- Buffer rows must be 256-byte aligned (wgpu spec requirement)
- Must call device.poll() BEFORE awaiting map_async future (otherwise freezes)
- Must drop mapped range before calling unmap()
- Can't read texture directly - must copy to buffer first
- Use Rgba8UnormSrgb format for UI rendering (matches typical framebuffer)

## CI Integration Pattern

```yaml
# .github/workflows/visual-tests.yml
name: Visual Regression Tests

on: [push, pull_request]

jobs:
  visual-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-insta
        run: cargo install cargo-insta --version 1.46.1

      - name: Run visual regression tests
        run: cargo insta test --check
        # --check fails if snapshots don't match (no .snap.new files)

      - name: Upload snapshot diffs (on failure)
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: snapshot-diffs
          path: |
            **/*.snap.new
            **/*.snap
          retention-days: 7

      # If tests fail:
      # 1. Download artifact from GitHub Actions UI
      # 2. Review .snap.new files locally with cargo-insta
      # 3. Run `cargo insta review` to accept/reject
      # 4. Commit accepted snapshots
```

**Local workflow:**
```bash
# Make UI changes
# ...

# Run tests (generates .snap.new for changes)
cargo test

# Review snapshots interactively
cargo insta review
# Shows diff, prompts: [a]ccept, [r]eject, [s]kip

# Or auto-accept all (careful!)
cargo insta accept

# Commit accepted snapshots
git add **/*.snap
git commit -m "Update visual snapshots for sidebar fix"
```

## Threshold Selection Guidance

Based on Tony Finn's screenshot testing blog and image-compare/dssim docs:

| Comparison Type | Recommended Tool | Threshold | Why |
|-----------------|-----------------|-----------|-----|
| Text rendering | image-compare SSIM | 0.005-0.01 | Text has high-frequency details. Font hinting varies across platforms. Subpixel rendering differs. |
| UI shapes/borders | image-compare SSIM | 0.001-0.005 | Geometric shapes should be consistent. Stricter threshold catches regressions. |
| Color/gradients | dssim | 0.0005 | L*a*b* color space better matches human perception. Multiscale catches banding. |
| Pixel-art/icons | image-compare RMS | < 1% pixels diff | When SSIM is too forgiving. RMS catches single-pixel changes. |

**Start loose, tighten over time:** Begin with threshold 0.01, run tests across CI environments, tighten to 0.005 once stable. Cross-GPU variance will dictate practical limit (typically 0.001-0.005 for SSIM).

## Known Issues & Mitigations

### Issue: Cross-GPU determinism
**Problem:** Different GPUs (NVIDIA vs AMD vs Intel) produce slightly different RGB values for same rendering code. Even same vendor with different driver versions can vary.

**Impact:** Flaky CI tests, false positives.

**Mitigation:**
- Use SSIM (structural similarity) instead of pixel-perfect comparison
- Set threshold around 0.005-0.01 based on empirical testing
- Consider separate snapshot sets per GPU vendor (complex, last resort)
- Use software rendering (wgpu backend: Vulkan Swiftshader or DX12 WARP) for determinism at cost of speed

### Issue: Text rendering platform differences
**Problem:** Glyphon uses different rasterizers on different platforms. Font hinting varies. Subpixel rendering differs.

**Impact:** Same text looks different on Windows vs Linux vs macOS.

**Mitigation:**
- Disable font hinting in glyphon (use hinting: FontHinting::None)
- Use same font files across platforms (bundle fonts, don't use system fonts)
- Capture at integer pixel positions (avoid fractional positioning)
- Accept higher threshold for text (0.01) vs geometric shapes (0.005)

### Issue: CI environment lacks GPU
**Problem:** GitHub Actions Linux runners have no GPU. wgpu requires GPU or software fallback.

**Impact:** Tests can't run without backend.

**Mitigation:**
- Use wgpu software backends: Vulkan with Swiftshader or DX12 with WARP
- Set environment variables: `WGPU_BACKEND=vulkan` with Swiftshader installed
- Or use CPU-based rendering for tests (slower but deterministic)
- See wgpu CI examples in official repo for reference configs

### Issue: Snapshot bloat
**Problem:** Binary PNG files in git repo grow large over time.

**Impact:** Slow clones, large repo size.

**Mitigation:**
- Use git LFS for .snap files if repo grows > 50MB
- Or store snapshots in separate repo/artifact storage
- Compress PNGs aggressively (pngcrush/oxipng)
- Only snapshot critical UI states (not every permutation)

## Sources

### HIGH Confidence (Official docs, Context7, verified)
- [insta GitHub repository](https://github.com/mitsuhiko/insta) — Latest version 1.46.1, CI integration docs
- [Insta documentation](https://insta.rs/) — Official docs for snapshot testing patterns
- [cargo-insta CLI docs](https://insta.rs/docs/cli/) — CI integration, environment variables
- [image-compare GitHub](https://github.com/ChrisRega/image-compare) — Version 0.5.0, SSIM/RMS algorithms
- [dssim GitHub](https://github.com/kornelski/dssim) — Version 3.4.0, perceptual similarity
- [Learn Wgpu windowless tutorial](https://sotrh.github.io/learn-wgpu/showcase/windowless/) — Authoritative wgpu texture readback example
- [wgpu official docs](https://docs.rs/wgpu/latest/wgpu/) — Texture, Buffer, copy operations

### MEDIUM Confidence (Blog posts verified with official sources)
- [Tony Finn: Screenshot testing with Rust](https://tonyfinn.com/blog/rust-screenshot-testing/) — Good patterns, but xray crate unmaintained (use concepts, not library)
- [LogRocket: Using Insta for snapshot testing](https://blog.logrocket.com/using-insta-rust-snapshot-testing/) — Practical usage examples
- [GitHub Actions Rust CI patterns](https://docs.github.com/en/actions/tutorials/build-and-test-code/rust) — Official GitHub docs

### LOW Confidence (Web search only, needs validation)
- Cross-GPU variance: Multiple sources mention this issue but no authoritative documentation found. Empirical testing needed to determine acceptable thresholds for your specific hardware/driver matrix.

---
*Stack research for: Visual regression testing for GPU-rendered IDE UI in Rust*
*Researched: 2026-01-28*
