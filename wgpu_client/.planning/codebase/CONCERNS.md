# Codebase Concerns

**Analysis Date:** 2026-01-28

## Tech Debt

**Unimplemented File Operations:**
- Issue: `file.new` and `file.open` commands have TODO stubs with no implementation
- Files: `src/app.rs` (lines 254, 257)
- Impact: Core file management functionality is blocked. Users cannot create new files or open files through the UI
- Fix approach: Implement file dialogs and file creation logic by dispatching to the core editor

**Blocking GPU Initialization:**
- Issue: `pollster::block_on()` is called on the main thread to create the GPU renderer, blocking event loop during initialization
- Files: `src/app.rs` (line 612)
- Impact: On slow hardware or with large projects, window creation could freeze the UI. Async initialization would be better but requires architectural changes
- Fix approach: Migrate GPU initialization to be fully async, handling renderer creation failure gracefully without blocking

**Hardcoded Layout Constants:**
- Issue: Magic numbers scattered throughout layout calculations (status bar height = 24.0, tab bar heights, sidebar widths)
- Files: `src/app.rs` (lines 296, 298, 319-320, 394), `src/renderer/mod.rs`
- Impact: Layout changes require hunting through multiple files. No single source of truth for layout dimensions
- Fix approach: Extract all layout constants to a centralized config module (e.g., `src/design_system/tokens/layout.rs`)

**Excessive Unwrap_or Usage:**
- Issue: 43+ instances of `.unwrap_or()` with fallback defaults throughout codebase
- Files: `src/app.rs`, `src/components/ui_text_renderer.rs`, `src/design_system/primitives/styled_rect.rs`, `src/renderer/mod.rs`, `src/ui/command_palette.rs`, `src/widgets/`
- Impact: Silent failures when values are None. Examples: font metrics defaulting to 0.0, colors defaulting to transparent, counts defaulting to 1. Makes debugging subtle rendering issues harder
- Fix approach: Replace with explicit error handling or validation. Document why defaults are safe where they're unavoidable

**Floating Point Comparison:**
- Issue: `partial_cmp()` used with `unwrap_or()` on NaN-prone floating point operations
- Files: `src/components/ui_text_renderer.rs` (line 191)
- Impact: If any layout run produces NaN (zero width text, invalid metrics), comparison fails silently and returns Equal, potentially causing layout errors
- Fix approach: Add NaN checks before comparison, or use a safe floating point comparison utility

## Known Bugs

**Double-Click Detection Gap:**
- Symptoms: Position tracking uses old click data on hover, could trigger false double-clicks during rapid cursor movement without clicking
- Files: `src/app.rs` (lines 567-584)
- Trigger: Move mouse rapidly, click, then immediately move again within 500ms and 5 pixels
- Workaround: None - inherent to the implementation

**Incomplete Drag Selection:**
- Symptoms: Text drag selection is flagged `is_dragging` but drag events may be blocked during dialog operations without unsetting the flag
- Files: `src/app.rs` (lines 684, 668)
- Trigger: Start text drag selection, then trigger dialog (e.g., open file), close dialog
- Workaround: Click elsewhere to reset drag state

**Scale Factor Consistency:**
- Symptoms: Scale factor cast from `f64` to `f32` multiple times, potential precision loss if monitors have unusual DPI
- Files: `src/app.rs` (lines 113, 293, 394, 541, 675)
- Trigger: High-DPI monitors (> 2.0 scale), sub-pixel rendering precision matters
- Workaround: None - would need unified scale factor management

## Security Considerations

**File Path Handling:**
- Risk: No path validation on file operations. Relative paths and symlinks not explicitly handled
- Files: `src/app.rs` (lines 65, 83)
- Current mitigation: Relies on core_editor for validation
- Recommendations: Add explicit path normalization, prevent directory traversal (../), document supported path types

**GPU Memory Exhaustion:**
- Risk: Out-of-memory errors logged but not handled gracefully. Could crash or hang if texture atlas fills up
- Files: `src/app.rs` (line 144)
- Current mitigation: Error is logged, then ignored and frame continues
- Recommendations: Implement memory budgeting, graceful degradation (reduce texture quality), or shutdown sequence

**Window Creation Error Handling:**
- Risk: Window creation errors use `.expect()` which panics without cleanup
- Files: `src/app.rs` (line 608)
- Current mitigation: Application terminates
- Recommendations: Return error gracefully, attempt retry, provide user feedback

## Performance Bottlenecks

**Pre-allocated Buffer Limits:**
- Problem: Hard-coded limits on UI text blocks (256), rectangle instances (4096), and similar structures
- Files: `src/components/ui_text_renderer.rs` (line 14), `src/components/rect.rs` (line 56), `src/design_system/primitives/styled_rect_renderer.rs`
- Cause: If UI renders more elements than limits, they silently don't render or crash. No overflow handling
- Improvement path: Implement dynamic allocation with growth, or use multiple batches if limits exceeded

**Frame Rendering Overhead:**
- Problem: Every frame, all UI components are re-measured and re-laid out from scratch
- Files: `src/renderer/mod.rs` (entire frame cycle), `src/app.rs` (render calls)
- Cause: No caching of layout results between frames when input hasn't changed
- Improvement path: Implement layout caching invalidated only when viewport changes or state updates

**String Cloning in Layout:**
- Problem: Multiple `.clone()` calls on file names, text content, and command entries per frame
- Files: `src/app.rs` (lines 191, 194, 375, 378), `src/renderer/mod.rs` (lines 380, 447)
- Cause: No lifetime management for temporary strings used during layout
- Improvement path: Use references or string interning for frequently cloned values

**Gutter Width Recalculation:**
- Problem: Gutter width is recalculated every frame based on line count
- Files: `src/renderer/mod.rs` (line 903)
- Cause: `.to_string().len()` called on line numbers repeatedly
- Improvement path: Cache digit count, only recalculate when line count crosses digit boundaries (10->99, 100->999)

## Fragile Areas

**Mouse Input Event Ordering:**
- Files: `src/app.rs` (lines 289-585)
- Why fragile: Multiple state flags (`last_click_time`, `last_click_pos`, `is_dragging`, `cursor_position`) updated asynchronously by different events. Order-dependent logic for double-click detection
- Safe modification: Add comprehensive state machine validation tests, ensure all paths clear flags properly
- Test coverage: Only double-click detection exists, drag state transitions not tested

**GPU Renderer State:**
- Files: `src/renderer/mod.rs` (entire module, 928 lines)
- Why fragile: Central coordination point for 17+ sub-components. Each component holds device/queue references and GPU state. Changes to one component's layout can cascade
- Safe modification: Add wrapper methods for state updates, create integration tests for component interactions
- Test coverage: Minimal - only animation and layout components have unit tests

**Dynamic Text Rendering:**
- Files: `src/components/ui_text_renderer.rs` (209 lines)
- Why fragile: Glyphon buffer management with pre-allocated fixed counts. `prepare()` silently truncates if text blocks exceed MAX_TEXT_BLOCKS (256)
- Safe modification: Add overflow detection and logging, test with large text block arrays
- Test coverage: No tests for overflow conditions or metric calculations

**Layout Constraint Propagation:**
- Files: `src/design_system/layout/flex.rs` (457 lines)
- Why fragile: Flex layout calculations depend on parent constraints trickling down correctly. No validation that constraints are monotonic (min <= max)
- Safe modification: Add constraint validation, write layout test suite with invalid constraint inputs
- Test coverage: Only basic flex direction and alignment tested

## Scaling Limits

**Text Block Rendering Capacity:**
- Current capacity: 256 text blocks per frame (hardcoded in UITextRenderer)
- Limit: When UI renders > 256 text elements (long file trees, large dialogs with many items), excess blocks silently don't render
- Scaling path: Implement batching system that handles overflow, or use dynamic allocation

**Rectangle Rendering Capacity:**
- Current capacity: 4096 rectangles per frame (hardcoded in RectRenderer)
- Limit: Complex nested layouts with many highlight/selection/background rects could exceed this
- Scaling path: Dynamic buffer reallocation on demand, or implement multi-batch rendering

**GPU Texture Atlas:**
- Current capacity: Single text atlas created at initialization
- Limit: Large projects with many unique characters/fonts could exhaust atlas memory
- Scaling path: Implement texture atlas growth or streaming, fallback to software rendering for overflow

**File Tree Depth:**
- Current capacity: No explicit limit
- Limit: Very deep directory structures could cause stack overflow in recursive rendering or consume excessive layout memory
- Scaling path: Implement iterative depth tracking, lazy loading of nested items

## Dependencies at Risk

**wgpu 23 (GPU Framework):**
- Risk: wgpu is pre-1.0 and API-breaking changes are possible between minor versions. Version pinned to 23, not 0.23
- Impact: Cargo.toml shows `wgpu = "23"` which likely means wgpu 0.23.x (semver caret). If 0.24 introduces breaking changes and gets pulled in, build breaks
- Migration plan: Monitor wgpu releases, implement feature flags for API compatibility, consider pinning to exact version

**glyphon 0.7 (Text Rendering):**
- Risk: Early-stage text library with potential API instability
- Impact: Text rendering could break, performance regression in glyph cache
- Migration plan: Evaluate cosmic-text as alternative (more stable), implement abstraction layer for text renderer

**winit 0.30 (Window Management):**
- Risk: Event API and window creation could change
- Impact: High - central to application, would require rewrite of event loop
- Migration plan: Pin to 0.30.x range, monitor release notes

## Missing Critical Features

**File Persistence:**
- Problem: File operations (new, open, save) are stubbed out with no implementation
- Blocks: Cannot save work, cannot open existing files, basic file management broken

**Undo/Redo for Layout Changes:**
- Problem: UI state changes (drag sidebar, toggle panels) don't participate in undo/redo system
- Blocks: Complex workflows can't be undone; only text editing can be reverted

**Viewport Caching:**
- Problem: viewport dimensions cached but invalidation logic not clear
- Blocks: Efficient multi-monitor support, smooth window resizing

**Error Recovery:**
- Problem: GPU errors (OutOfMemory, Lost surface) only log without recovery
- Blocks: Long-running sessions on low-memory systems will degrade and fail

## Test Coverage Gaps

**UI Event Integration:**
- What's not tested: Mouse input handling, keyboard event routing, focus management across components
- Files: `src/app.rs`, `src/renderer/mod.rs`
- Risk: Regressions in input behavior (drag selection, double-click, modifier key handling) go undetected
- Priority: High - input is core to editor usability

**Renderer Component Interactions:**
- What's not tested: Layout constraints propagation, text block rendering with overflow, GPU memory pressure
- Files: `src/renderer/mod.rs` (928 lines), `src/components/ui_text_renderer.rs`
- Risk: Silent rendering failures when limits exceeded, subtle layout bugs with complex component nesting
- Priority: High - rendering is critical path

**Scale Factor Handling:**
- What's not tested: DPI changes, high-DPI monitors (2.0+), mixed-scale multi-monitor setups
- Files: `src/app.rs`, `src/renderer/mod.rs`
- Risk: Layout corruption, text rendering artifacts on unusual display configurations
- Priority: Medium - affects specific hardware configurations

**GPU Initialization Failure:**
- What's not tested: Fallback when GPU unavailable, graceful degradation on weak hardware
- Files: `src/renderer/mod.rs`, `src/app.rs`
- Risk: Crash on unsupported hardware, no error message to user
- Priority: Medium - affects hardware compatibility

---

*Concerns audit: 2026-01-28*
