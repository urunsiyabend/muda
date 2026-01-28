# Technology Stack

**Analysis Date:** 2026-01-28

## Languages

**Primary:**
- Rust 2024 edition (Edition 2024) - Core language for entire project

**Runtime:**
- rustc 1.90.0 (1159e78c4 2025-09-14) - Nightly/experimental Rust compiler

## Runtime & Package Manager

**Environment:**
- Cargo workspace with 3 crates: `core_editor`, `ratatui_client`, `wgpu_client`
- Rust stable toolchain (1.70+ recommended per README)

**Package Manager:**
- Cargo - Rust's native dependency manager
- Lockfile: `Cargo.lock` present

## Frameworks & Libraries

**Core Editor Library (`core_editor`):**
- `ropey` (1.6.1) - Rope-based text buffer implementation
- `tree-sitter` (0.26.3) - Incremental parsing for syntax highlighting
  - Language parsers: `tree-sitter-rust` (0.24.0), `tree-sitter-python` (0.25.0), `tree-sitter-javascript` (0.25.0), `tree-sitter-json` (0.24.8), `tree-sitter-toml` (0.20.0), `tree-sitter-md` (0.5.1)
- `arboard` (3.6.1) - Clipboard access (Windows, macOS, Linux)

**Ratatui TUI Client (`ratatui_client`):**
- `ratatui` (0.29.0) - Terminal UI framework
- `crossterm` (0.29.0) - Terminal input/output handling
  - Used for: raw mode, alternate screen, bracketed paste events

**WGPU GPU Client (`wgpu_client`):**
- `wgpu` (23) - GPU rendering abstraction (WebGPU standard)
- `winit` (0.30) - Cross-platform windowing and event handling
- `glyphon` (0.7) - Font rendering and text layout
- `pollster` (0.4) - Async runtime helper for blocking on futures

**Utilities:**
- `log` (0.4.29) - Logging facade
- `simplelog` (0.12.2) - Simple logging implementation to files
- `anyhow` (1.0.100) - Error handling with context
- `bytemuck` (1.21) - Zero-copy type casting with derive support

**Development & Benchmarking:**
- `criterion` (0.5 with HTML reports) - Benchmarking framework

## Key Dependencies Breakdown

### Text Handling
- **ropey**: Implements CRDT-like rope data structure for efficient large-file text editing
- **tree-sitter**: Enables incremental, language-aware syntax highlighting with parser bindings for 6+ languages

### Graphics & Display
- **wgpu**: Cross-platform GPU abstraction with Vulkan/Metal/DirectX backends
- **glyphon**: Hardware-accelerated text rendering with font atlas management
- **ratatui**: Declarative TUI rendering with layout system

### System Integration
- **winit**: Event loop management, input capture, window lifecycle
- **crossterm**: Terminal capability detection and ANSI control sequences
- **arboard**: System clipboard access (cross-platform)

### Error & Logging
- **anyhow**: Context-preserving error propagation
- **log/simplelog**: Structured logging to `editor.log`

## Configuration

**Environment:**
- No `.env` file configuration detected
- File-based: Documents opened from command-line arguments
- Logging configured programmatically in `main.rs`
  - TUI client: `WriteLogger` to `editor.log` at `Debug` level
  - GPU client: `WriteLogger` to `editor.log` at `Debug` level

**Build Configuration:**
- Workspace-level: `resolver = "2"`, edition 2024
- Per-crate features managed via Cargo.toml
- No custom build scripts detected

## Platform Requirements

**Development:**
- Rust 1.70+ (stable)
- Edition 2024 support (requires nightly or future stable)
- Operating system: Windows, macOS, or Linux
- GPU client requires graphics driver support (Vulkan/Metal/DirectX)

**Production:**
- Deployment target: Desktop (native binaries only, no web/mobile)
- Two binary outputs:
  - `muda-tui` - Terminal UI via ratatui + crossterm
  - `muda` - GPU-accelerated client via wgpu + winit + glyphon
- Both share core domain logic from `core_editor` library

**File I/O:**
- Standard file system operations via `std::fs`
- Document persistence: Read/write ANSI text files with line-ending detection (LF, CRLF, CR)
- Clipboard access via arboard

**Graphics Requirements (GPU Client):**
- GPU with WebGPU support (Vulkan, Metal, DirectX 12+)
- Font rendering via system fonts (glyphon manages atlas)

---

*Stack analysis: 2026-01-28*
