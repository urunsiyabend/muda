# Technology Stack

**Analysis Date:** 2026-01-28

## Languages

**Primary:**
- Rust (Edition 2024, Rust 1.90.0) - All core codebase, including editor logic, rendering, and UI

**Secondary:**
- None detected

## Runtime

**Environment:**
- Native compiled binaries (no runtime VM)
- Cross-platform: Windows, Linux, macOS via Rust stdlib and library support

**Package Manager:**
- Cargo (1.90.0)
- Lockfile: `./Cargo.lock` present (3757 lines of pinned dependencies)

## Frameworks

**Core:**
- wgpu (v23) - GPU rendering abstraction (Metal, Vulkan, DX12, GL)
  - Location: `wgpu_client` crate
  - Used for: Graphics pipeline, surface management, shader execution

- winit (v0.30) - Window management and event loop
  - Location: `wgpu_client/src/main.rs`
  - Used for: OS window creation, input handling, event dispatch

- glyphon (v0.7) - Text rendering and glyph rasterization
  - Location: `wgpu_client` dependencies
  - Used for: Rendering text to GPU textures

- ratatui (v0.29.0) - Terminal UI framework
  - Location: `ratatui_client` crate
  - Used for: TUI rendering alternative to GPU client

- tree-sitter (v0.26.3) - Code parsing and syntax highlighting
  - Location: `core_editor` crate
  - Used for: Language-aware parsing and syntax highlighting

**Testing:**
- criterion (v0.5, dev-dependencies) - Benchmarking framework
  - Location: `core_editor/Cargo.toml`
  - Config: Generates HTML reports
  - Benchmarks: 4 benchmark suites in `core_editor/benches/`

**Build/Dev:**
- Cargo (built-in) - Build system and dependency management

## Key Dependencies

**Critical:**
- ropey (v1.6.1) - Efficient copy-on-write rope data structure for text buffers
  - Why: Core to text model performance; enables efficient large document edits
  - Location: `core_editor`

- arboard (v3.6.1) - Clipboard access (cross-platform)
  - Why: Copy/paste functionality
  - Location: `core_editor`

- tree-sitter-rust (v0.24.0) - Rust language parser
- tree-sitter-python (v0.25.0) - Python language parser
- tree-sitter-javascript (v0.25.0) - JavaScript language parser
- tree-sitter-json (v0.24.8) - JSON language parser
- tree-sitter-toml (v0.20.0) - TOML language parser
- tree-sitter-md (v0.5.1) - Markdown language parser
  - Why: Syntax highlighting for multiple languages
  - Location: `core_editor`

**Infrastructure:**
- log (v0.4) - Logging facade
  - Location: Used across `core_editor`, `wgpu_client`, `ratatui_client`

- simplelog (v0.12) - Concrete logging implementation
  - Why: File-based logging to `editor.log`
  - Location: `wgpu_client/src/main.rs`, `ratatui_client/src/main.rs`

- anyhow (v1.0) - Error handling with context
  - Why: Ergonomic error propagation
  - Location: `wgpu_client/src/main.rs`

- bytemuck (v1.21) - Zero-copy casting for GPU data
  - Features: `derive` feature enabled for macro support
  - Why: Efficient struct-to-GPU memory mapping
  - Location: `wgpu_client`

- pollster (v0.4) - Lightweight async runtime
  - Why: Blocking on async wgpu calls
  - Location: `wgpu_client`

- crossterm (v0.29.0) - Terminal manipulation (TUI)
  - Why: Cross-platform terminal input/output for ratatui
  - Location: `ratatui_client`

## Configuration

**Environment:**
- Command-line arguments only (no `.env` files detected)
- Entry point accepts path argument: `cargo run -- <file_or_directory>`
  - File path: Opens editor with sidebar hidden
  - Directory path: Opens editor with file explorer sidebar visible
  - No args: Opens empty editor

**Build:**
- Standard Cargo build configuration
- No custom build scripts detected
- Release builds: `cargo build --release`
- Debug builds: `cargo build`

## Crates Structure

**Workspace Members:**
1. `core_editor` - Library crate (no binary)
   - Path: `./core_editor/`
   - Purpose: Core domain logic, text buffer, syntax highlighting
   - No external API dependencies

2. `ratatui_client` - Binary crate
   - Path: `./ratatui_client/`
   - Binary name: `muda-tui`
   - Purpose: Terminal UI frontend
   - Entry point: `ratatui_client/src/main.rs`

3. `wgpu_client` - Binary crate
   - Path: `./wgpu_client/`
   - Binary name: `muda`
   - Purpose: GPU-accelerated desktop UI frontend
   - Entry point: `wgpu_client/src/main.rs`

## Platform Requirements

**Development:**
- Rust 1.70+ (stable toolchain recommended)
- Cargo package manager
- Platform-native graphics drivers (for wgpu backends)

**Production (wgpu_client):**
- Windows: DirectX 12 or Vulkan
- macOS: Metal (10.13+)
- Linux: Vulkan or OpenGL 4.3+
- Minimum GPU: Any supported by wgpu (most modern GPUs)

**Production (ratatui_client):**
- Any terminal emulator (Windows conhost, Unix terminals)
- No GPU required

## Logging

**Framework:** log + simplelog

**Configuration:**
- Sinks to file: `./editor.log`
- Log level: Debug
- Initialization in `wgpu_client/src/main.rs` and `ratatui_client/src/main.rs`

**Code:**
```rust
// From wgpu_client/src/main.rs
let log_file = File::create("editor.log")?;
WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)?;
```

---

*Stack analysis: 2026-01-28*
