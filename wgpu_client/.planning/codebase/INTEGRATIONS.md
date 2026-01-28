# External Integrations

**Analysis Date:** 2026-01-28

## APIs & External Services

**Not detected:** No external HTTP APIs, cloud services, or third-party integrations.

This is a self-contained editor with no external service dependencies.

## Data Storage

**Databases:**
- Not used. The editor does not use a database.

**File Storage:**
- Local filesystem only
- Editor reads and writes user files directly to disk
- Commands: `WgpuApp::open_file()`, `WgpuApp::open_directory()` in `wgpu_client/src/app.rs`
- Saved documents written to their original file paths

**Caching:**
- In-memory only (during application runtime)
- No persistent cache layer detected
- Text buffer cached in memory via ropey data structure

## Authentication & Identity

**Auth Provider:**
- Not applicable. No user authentication system.
- Single-user local application.

## Monitoring & Observability

**Error Tracking:**
- None. No external error reporting service detected.

**Logs:**
- Local file only: `./editor.log`
- Implementation: Rust `log` crate with `simplelog` backend
- Configuration in:
  - `wgpu_client/src/main.rs`
  - `ratatui_client/src/main.rs`
- Log level: Debug
- Code example:
  ```rust
  let log_file = File::create("editor.log")?;
  WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)?;
  ```

## CI/CD & Deployment

**Hosting:**
- Not applicable. Desktop application (native binary).
- Users compile from source: `cargo build --release`

**CI Pipeline:**
- Not detected. No GitHub Actions, GitLab CI, or other CI configuration found.

**Deployment:**
- Manual build and run
- Binary execution: `./target/release/muda` (wgpu) or `./target/release/muda-tui` (ratatui)

## Environment Configuration

**Required env vars:**
- None detected. Application uses command-line arguments only.

**Optional env vars:**
- Rust debug configuration (RUST_LOG, RUST_BACKTRACE) for development
- Not built into the application

**Secrets location:**
- Not applicable. No secrets or API keys required.

## System Integration

**Clipboard:**
- Library: arboard (v3.6.1) in `core_editor`
- Purpose: Copy/paste operations
- Integration: `arboard::Clipboard::new()` for cross-platform clipboard access

**Window Management:**
- Library: winit (v0.30)
- Purpose: OS window creation and event handling
- Integration: `winit::event_loop::EventLoop` in `wgpu_client/src/main.rs`

**Graphics API:**
- Library: wgpu (v23)
- Backends: Vulkan, Metal, DX12, OpenGL (automatic selection)
- Purpose: GPU-accelerated rendering pipeline
- Configuration: `wgpu_client/src/renderer/mod.rs` (lines 47-98)
- Surface configuration: Automatic detection of best available backend

## Webhooks & Callbacks

**Incoming:**
- Not applicable. Desktop application.

**Outgoing:**
- Not applicable. No external service calls.

## Plugin/Extension System

**Current Status:**
- Not implemented. Application is monolithic.
- README notes extensibility as design goal but not yet realized
- Architecture designed to support future plugin system via event-driven architecture

## Third-Party Services

**Code Parsing:**
- tree-sitter (v0.26.3) - Standalone parsing library
- Language support: Rust, Python, JavaScript, JSON, TOML, Markdown
- No external parser service; parsing runs locally

---

*Integration audit: 2026-01-28*
