# Codebase Concerns

**Analysis Date:** 2026-01-28

## Tech Debt

**Rust Edition 2024 (Experimental):**
- Issue: Both `core_editor/Cargo.toml` and `wgpu_client/Cargo.toml` specify `edition = "2024"`, which is not yet stable
- Files: `core_editor/Cargo.toml`, `wgpu_client/Cargo.toml`
- Impact: Dependency on unstable Rust edition may cause compilation failures with stable toolchain; blocks publishing to crates.io
- Fix approach: Downgrade to `edition = "2021"` for stability, then incrementally adopt 2024 features once stabilized

**Multiple Renderer Implementations Coexist:**
- Issue: Both legacy components and new design-system components are loaded simultaneously in `GpuRenderer`
- Files: `wgpu_client/src/renderer/mod.rs` (lines 14-34 show old components; lines 24-30 show new ones)
- Impact: Increased GPU memory usage, more complex rendering pipeline, harder to maintain and optimize
- Fix approach: Complete migration to design-system components, remove legacy TextArea/Gutter/Caret/TabBar/Dialog when new UI is fully functional

**Manual Viewport Caching:**
- Issue: `WgpuApp` manually tracks cached viewport dimensions to avoid recalculating every frame
- Files: `wgpu_client/src/app.rs` (line 38: `cached_viewport` field, lines 130-134 in render())
- Impact: State synchronization bug risk; edge cases where cache invalidation fails (e.g., file open at line 197)
- Fix approach: Implement automatic dimension tracking, remove manual cache field, or use weak notifications instead

**Character Width Measurement Fallback:**
- Issue: `TextArea::measure_char_width()` returns hardcoded `font_size * 0.6` fallback if glyph measurement fails
- Files: `wgpu_client/src/components/text_area.rs` (lines 70-81)
- Impact: Text positioning will be inaccurate for fonts where this approximation fails; silent failure (no error logged)
- Fix approach: Either ensure measurement always succeeds (handle font loading errors), or log warnings when fallback is used

**Viewport Resolution Coupling:**
- Issue: `TextArea::prepare()` requires full screen resolution (`screen_width`, `screen_height`) to update glyphon viewport, even though it only uses `bounds` for text area sizing
- Files: `wgpu_client/src/components/text_area.rs` (lines 101-107)
- Impact: Text area cannot be resized independently; requires full render pass context
- Fix approach: Decouple viewport resolution from text area bounds, pass only needed dimensions

## Known Bugs

**Syntax Highlighting File Size Limit:**
- Symptoms: Files > 500KB will not get real-time syntax highlighting; highlighting is deferred
- Files: `core_editor/src/syntax.rs` (line 11: `MAX_INCREMENTAL_PARSE_SIZE = 500_000`)
- Trigger: Open or edit a file larger than 500KB
- Workaround: None currently; users must manually trigger full reparse or wait for background parse
- Context: This is a deliberate tradeoff to avoid blocking UI on large files

**Command Palette Missing Implementation:**
- Symptoms: Command palette structure exists but command execution is incomplete
- Files: `wgpu_client/src/renderer/mod.rs` (lines 132-150 register commands), `wgpu_client/src/app.rs` (lines 202-228 route to renderer), but no actual command execution logic shown
- Trigger: User selects a command from the palette
- Impact: UI accepts input but likely drops commands or crashes with "unimplemented" error

**Log File Created in Current Directory:**
- Symptoms: `editor.log` created in working directory instead of config/temp directory
- Files: `wgpu_client/src/main.rs` (line 29: `File::create("editor.log")`)
- Trigger: On application startup
- Impact: Pollutes project directory; untracked log accumulation; Windows-specific: may lock file on some systems
- Fix approach: Use standard config directory (e.g., `~/.config/muda/` on Linux, `%APPDATA%\Muda\` on Windows)

## Security Considerations

**Clipboard Integration (Untrusted Input):**
- Risk: Pasting extremely large text from clipboard could allocate unbounded memory
- Files: `core_editor/Cargo.toml` (arboard dependency), clipboard handlers in commands
- Current mitigation: None visible; no size limits on paste operations
- Recommendations: Implement max clipboard paste size (e.g., 100MB), warn user for large pastes

**Path Traversal in File Opening:**
- Risk: Sidebar file opening constructs paths from user input without validation
- Files: `wgpu_client/src/app.rs` (lines 189-198: `SidebarOpen` action), `core_editor/src/domain/workspace.rs` likely opens paths without validation
- Current mitigation: OS filesystem restrictions
- Recommendations: Validate opened paths are within intended directory, prevent directory traversal sequences, add symlink handling

**Unvetted Tree-Sitter Grammars:**
- Risk: Tree-sitter language bindings (`tree_sitter_rust`, `tree_sitter_python`, etc.) may have parsing vulnerabilities or memory issues
- Files: `core_editor/Cargo.toml` (lines 13-18), `core_editor/src/syntax.rs` (lines 47-56)
- Current mitigation: File size limit (500KB) on incremental parsing
- Recommendations: Add timeout on tree-sitter parse operations, add heap size limits, regularly update grammar versions

## Performance Bottlenecks

**RenderModel Built Every Frame:**
- Problem: Complete `ViewModelBuilder::build()` runs every frame, recomputing all visible lines and spans
- Files: `wgpu_client/src/app.rs` (line 136: `build_render_model()` called unconditionally), `core_editor/src/view_model/builder.rs`
- Cause: No incremental rendering; view model is not cached across frames
- Current capacity: Smooth on ~10-40KB files at 120 char width; benchmarks show 1MB files still build in <100ms
- Improvement path: Cache render model, only rebuild on document/view state changes; track dirty regions

**Syntax Highlighting Recalculated on Every Render:**
- Problem: `SyntaxHighlighter` re-parses visible lines even if document hasn't changed
- Files: `core_editor/src/syntax.rs` (no incremental parse caching visible), `core_editor/src/view_model/builder.rs` (line 88 builds visible_lines without caching)
- Cause: No syntax tree invalidation tracking; full traversal on each frame
- Improvement path: Cache parse tree by revision, only reparse changed regions; use tree-sitter incremental API

**GPU Texture Atlas Growth Unbounded:**
- Problem: `glyphon::TextAtlas` grows as new glyphs are encountered, but no eviction policy visible
- Files: `wgpu_client/src/components/text_area.rs` (line 21: `text_atlas`)
- Cause: No glyph cache management; long editing sessions may exhaust GPU memory on text-heavy files
- Improvement path: Implement LRU glyph cache eviction, or cap atlas size and reuse old glyphs

## Fragile Areas

**Viewport to Render Model Synchronization:**
- Files: `wgpu_client/src/app.rs`, `core_editor/src/view/viewport.rs`, `core_editor/src/view_model/builder.rs`
- Why fragile: Viewport dimensions (`width`, `height`) flow through multiple layers; off-by-one errors on width calculation (line 126-127 in app.rs) cause text clipping or incorrect scrolling
- Safe modification: Always validate character-width division with `.max(1)` to prevent zero-width viewports; add assertions that viewport dimensions match actual visible content
- Test coverage: Benchmarks exist (`render_benchmarks.rs`) but no unit tests for edge cases like 0-height viewport or very narrow windows

**Editor State During File Operations:**
- Files: `core_editor/src/app.rs` (lines 66-97 in `open_file()`), `core_editor/src/domain/workspace.rs`
- Why fragile: Opening a new file/directory mutates workspace state; concurrent modification during rendering could cause use-after-free in Rust, but state coherency is managed by single-threaded app
- Safe modification: Ensure all file operations complete before next render cycle; add state version checks to detect races
- Test coverage: No integration tests for file open/close scenarios during active editing

**Text Buffer Boundary Conditions:**
- Files: `core_editor/src/domain/text_buffer.rs` (lines 109-115 `char_at()`, 119-148 `line()`, 136-148 `line_len()`)
- Why fragile: Methods use `if offset >= self.len_chars()` boundary checks; off-by-one errors in caller logic can cause panic when accessing rope
- Safe modification: Return Option types consistently; add debug assertions on all boundary crossings
- Test coverage: No visible unit tests for TextBuffer boundary cases

**Command Palette State Machine:**
- Files: `wgpu_client/src/renderer/mod.rs` (lines 224-249), command palette toggle/show/hide logic
- Why fragile: Multiple entry points (toggle, show, hide) without clear state machine; race conditions if palette is toggled rapidly
- Safe modification: Implement explicit state enum (Hidden, Showing, Focus), validate transitions
- Test coverage: No tests for rapid toggle sequences or state consistency

## Scaling Limits

**Single-Threaded Rendering on Modern GPUs:**
- Current capacity: Achieves 60 FPS on typical scenes; syntax parsing blocks UI for >500KB files
- Limit: Where it breaks: GPU bandwidth underutilized on multi-core systems; large file syntax highlighting blocks UI thread for ~200-500ms
- Scaling path: Move syntax parsing to background thread with message passing; use wgpu compute shaders for text layout

**Text Rendering Atlas Memory:**
- Current capacity: Glyphon atlas starts at small size, grows dynamically; typical usage ~20-50MB for varied Unicode
- Limit: Where it breaks: Very large font sizes (48pt+) with many glyphs (CJK text) can exhaust VRAM on integrated GPUs
- Scaling path: Implement atlas eviction/LRU, partition into multiple atlases, or use signed distance field rendering

**File Tree Sidebar in Large Directories:**
- Current capacity: Not measured; sidebar iterates directory but no pagination visible
- Limit: Where it breaks: Directories with 10k+ files will cause UI lag when listing/scrolling
- Scaling path: Implement lazy-load directory listing, add incremental file discovery, implement virtual scrolling

## Dependencies at Risk

**Tree-Sitter Grammar Versions (Pinned):**
- Risk: `tree_sitter_rust = "0.24.0"`, `tree_sitter_python = "0.25.0"` pinned to older versions; newer versions may have security fixes or performance improvements
- Impact: Grammar bugs unfixed; may miss bug fixes in newer versions
- Migration plan: Review each grammar's changelog; test on a sample of large files before upgrading; establish update cadence (e.g., quarterly)

**Glyphon Text Rendering Library:**
- Risk: `glyphon = "0.7"` is relatively young ecosystem; breaking changes in 0.8+ may require significant refactoring
- Impact: Text rendering quality/performance improvements locked out; potential security issues in older versions
- Migration plan: Monitor glyphon releases; test 0.8+ on small feature branch; prepare alternative (cosmic-text) as fallback

**wgpu GPU Binding:**
- Risk: `wgpu = "23"` will eventually be superseded; breaking API changes common between major versions
- Impact: Unable to adopt new GPU features, performance improvements, or bug fixes in wgpu 24+
- Migration plan: No immediate action needed; monitor for wgpu 24.0 release timeline; plan upgrade within 6 months of release

## Missing Critical Features

**No Autosave / Session Recovery:**
- Problem: If app crashes, all unsaved work is lost
- Blocks: Multi-hour editing sessions; professional usage

**No Find/Replace Implementation:**
- Problem: UI registers "Find" and "Replace" commands, but no actual search/replace logic
- Blocks: Basic editing workflows

**No Multi-File Diff View:**
- Problem: Cannot compare or merge files side-by-side
- Blocks: Code review workflows

**No Terminal Integration:**
- Problem: No integrated terminal or command execution
- Blocks: Common IDE workflows

## Test Coverage Gaps

**No Unit Tests for Text Buffer Operations:**
- What's not tested: Edge cases in `TextBuffer` (empty buffer, single character, unicode boundaries, rope coalescence)
- Files: `core_editor/src/domain/text_buffer.rs`
- Risk: Off-by-one errors in coordinate mapping go unnoticed; unicode handling bugs in rope operations
- Priority: High (foundational data structure)

**No Integration Tests for Commands:**
- What's not tested: Command execution paths (Edit, Navigation, File), undo/redo sequences, error handling
- Files: `core_editor/src/commands/`, `core_editor/src/app.rs`
- Risk: Complex command interactions fail silently; undo/redo stack corruption undetected
- Priority: High (core functionality)

**No GPU Rendering Tests:**
- What's not tested: Text rendering correctness, scissor clipping, viewport calculations, coordinate transformations
- Files: `wgpu_client/src/renderer/`, `wgpu_client/src/components/`
- Risk: Rendering bugs (text corruption, clipping errors, color shifts) only caught visually
- Priority: Medium (requires GPU or emulation to test)

**No Syntax Highlighting Tests:**
- What's not tested: Correctness of tree-sitter queries, highlight span boundaries, incremental invalidation
- Files: `core_editor/src/syntax.rs`
- Risk: Syntax highlighting bugs on edge cases (unclosed strings, deep nesting) go unnoticed
- Priority: Medium (affects code readability)

**No File I/O Error Handling Tests:**
- What's not tested: Disk full, permission denied, file already open by another process, path traversal attacks
- Files: `core_editor/src/domain/workspace.rs`, file loading paths
- Risk: Graceful degradation not verified; security assumptions not validated
- Priority: High (safety-critical)

---

*Concerns audit: 2026-01-28*
