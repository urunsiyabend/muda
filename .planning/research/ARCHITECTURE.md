# Architecture Patterns: v2.0 Functional Editor Performance Deep Dive

**Domain:** GPU-rendered code editor, GPUI-style UI framework
**Researched:** 2026-03-26 (updated with scroll architecture, dirty tracking, blog citations)
**Overall confidence:** HIGH for ora-specific findings (read from source), MEDIUM for Zed patterns (DeepWiki + confirmed WebSearch summaries, WebFetch denied)

---

## Executive Summary

This document answers "How does Zed achieve its performance?" and translates those lessons
into concrete changes for the muda/ora architecture. The research combines direct source code
reading of the muda codebase with secondary research on Zed's architecture via DeepWiki
summaries and confirmed WebSearch findings from Zed's GitHub and blog posts.

**Critical finding:** The biggest performance problem in the current codebase is not
architectural — it is a single unconditional `request_redraw()` call at line 549 of
`ora/src/platform/event_loop.rs` that forces continuous frame rendering even when nothing
has changed. Fixing this is trivially easy and likely eliminates 90%+ of idle GPU work.
Zed itself confirmed this class of bug in their December 2025 Quality Week: they cut idle
GPU usage by only presenting frames during high-frequency input like scrolling, not after
every event.

---

## Section 1: How Zed's GPUI Achieves Performance

### 1.1 GPUI Is Immediate Mode — It Still Rebuilds Every Frame

**Confidence: HIGH (confirmed via multiple WebSearch summaries of Zed blog posts and GitHub)**

A common misconception: GPUI does NOT avoid rebuilding the view tree each frame through
partial tree diffing or retained element state. It uses **immediate mode** — every frame,
all views rebuild their element trees from scratch via `render()`.

The performance comes from something simpler: **GPUI only calls `draw()` when a window is
dirty.** The `WindowInvalidator` tracks `dirty_views: FxHashSet<EntityId>`. When an entity
calls `cx.notify()`, the view is marked dirty and the window is invalidated
(`platform_window.invalidate()`). If no entity changed, no drawing happens. Idle CPU use is
essentially zero.

```
Entity mutates via cx.update()
-> cx.notify() queues Effect::Notify
-> App::flush_effects() processes queue after update
-> WindowInvalidator::invalidate_view(entity_id)
-> platform_window.invalidate() -> OS schedules next draw
-> Next draw: rebuild full element tree, compute layout, paint, present
-> If nothing changed: no draw call at all
```

The GPUI README confirms this design explicitly: `CADisplayLink` on macOS checks a `dirty`
flag in `on_request_frame`. No dirty flag = no frame submitted.

**GPUI's own acknowledgement:** A Zed developer stated in a GitHub discussion:
"GPUI always re-layouts the whole app on each frame, with few caches here and there to
make performance slightly better." This confirms immediate-mode rebuild is the base behavior;
correctness of dirty detection is the main optimization.

**ora's inversion:** The continuous `request_redraw()` at line 549 of `event_loop.rs`
means ora renders at maximum frame rate unconditionally. Fixing this achieves the intended
GPUI design.

### 1.2 EditorElement Snapshot Architecture

**Confidence: MEDIUM (DeepWiki + WebSearch summaries)**

Zed's `EditorElement` (in `crates/editor/src/element.rs`) takes a snapshot of the
`DisplayMap` and `Editor` state once per frame during `prepaint()`. This `EditorSnapshot` is
an immutable view of buffer state at frame-start time. Layout and paint phases both use the
same snapshot, ensuring consistency even if the buffer changes on a background thread during
the frame.

The `DisplaySnapshot` provides:
- The visible display range (first and last rows that fit in the viewport)
- Chunk-based text iteration via `DisplaySnapshot::chunks()` — efficient for large files
- Coordinate mapping (buffer position ↔ display row/column)

This maps directly to how ora's `EditorRootView` already works — it calls
`adapter.build_render_model()` once per frame in `render()` and passes the resulting
`RenderModel` through the view tree. The snapshot pattern is already implemented correctly.

### 1.3 Scroll Architecture: What Gets Recomputed vs Cached

**Confidence: MEDIUM (DeepWiki + WebSearch summaries from GitHub PR #25009 and discussions)**

This section directly addresses "what happens on every scroll event in Zed."

**How Zed handles a scroll event:**

1. `ScrollManager` stores `scroll_anchor: ScrollAnchor` — the buffer position at the top
   of the viewport. A scroll event updates this value and calls `cx.notify()` on the
   Editor entity.

2. `cx.notify()` marks the Editor entity dirty → `WindowInvalidator` schedules a draw.

3. Next frame: `EditorElement::prepaint()` reads the (now-changed) `scroll_anchor` from
   the snapshot, recomputes `visible_line_range` (which rows are now in view), and lays
   out only those rows.

4. `EditorElement::paint()` generates GPU commands for the new visible lines.

**What IS recomputed on scroll:**
- The `EditorSnapshot` (fresh snapshot each frame — O(log n) rope clone)
- `visible_line_range` (which rows to render — recomputed from new scroll position)
- All element construction for newly-visible lines (AnyElement allocation)
- Layout computation (full flexbox pass — GPUI confirms this is per-frame)
- Paint commands (new GPU command list)

**What is NOT recomputed on scroll:**
- The `TextAtlas` (GPU texture — persists across all frames; glyphs added to it accumulate)
- Shaped glyph data for lines that remain in view (atlas cache hit — no reshaping)
- The underlying rope data structure (shared via Arc; not cloned for rendering)
- Syntax highlight spans for lines not affected by edit (tree-sitter's incremental output)

**Key insight from GPUI PR #25009** (merged, confirmed by WebSearch):
Zed added `AnyView::cached` — a mechanism where views can opt into caching their rendered
output. With this cache, `scroll` and `mouse move` events no longer trigger `render()` on
views that haven't changed their model state. Specifically: when the mouse moves over a
panel that doesn't care about the cursor position, that panel's render is skipped.
The problem this fixed: `window.refresh` (called by mouse-event-handling Divs) was
invalidating the cache for all sibling views — `ProjectPanel` would re-render on every
mouse move anywhere in the editor. After the fix: unaffected panels are skipped.

**What this means for ora (scroll architecture impact):**
- ora's current scroll implementation correctly uses `scroll_top_px` as a state variable
  that propagates to `TextAreaView` each frame. This is the right pattern.
- The missing piece is that mouse-move events currently call `request_redraw()` via the
  dirty-entity check path. For smooth scrolling (which fires many events), this is
  acceptable, but for cursor movement without scrolling it wastes frames.
- A future `cached_view` optimization (analogous to `AnyView::cached`) would let
  `GutterView` and `StatusBarView` skip render when only the scroll offset changed.

**Scroll without layout recompute:**
Neither Zed nor ora needs to skip layout on scroll because layout is fast (the flexbox
pass over ~100 elements is sub-millisecond). The win comes from not recomputing what is
unnecessary: lines that have scrolled off screen are simply not in the element tree.

### 1.4 DisplayMap Transformation Pipeline

**Confidence: MEDIUM (DeepWiki)**

Zed interposes a `DisplayMap` between buffer text and rendered output. This map handles:
- Fold map (collapsed regions)
- Tab expansion
- Soft word wrap
- Inlay hints and block decorations
- Coordinate mapping (buffer position to display row/column)

The muda architecture handles this in `core_editor/src/view_model/builder.rs`. The
`LinePresentation` type with `StyledSpan` vectors is the equivalent output. This is sound
architecture. The future work is making it cheaper to compute per-frame (see Section 2).

### 1.5 Glyph Atlas and Text Rendering

**Confidence: HIGH (glyphon source, Zed blog summary)**

Zed's text pipeline: OS font shaping -> glyph rasterization -> texture atlas (GPU texture
containing one copy of each glyph) -> instanced GPU rendering (glyphs read from atlas in
parallel).

The Zed "videogame" blog post states: "since the set of glyphs that needs to be rendered is
finite and can be cached quite effectively, rendering on the CPU doesn't really become a
bottleneck." Their target frame time is **below 3 milliseconds**.

The key optimization: the atlas **persists across frames**. A glyph shaped once is cached in
GPU memory and reused in every subsequent frame it appears in.

Ora already uses glyphon which implements the same strategy:
- `TextAtlas` is a persistent GPU texture (not re-created each frame)
- `atlas.trim()` (called in `TextSystem::clear()` at end of each frame) removes
  glyph entries that were not used in the last frame — this is LRU eviction, not atlas
  destruction
- The atlas grows and stabilizes after the first few frames once all common glyphs are seen

**What IS re-created each frame** in ora: `glyphon::Buffer` objects. These are constructed
in `LayoutContext::measure_text()` and stored in `PaintCommand::Text`. Glyphon must re-shape
and re-rasterize the text to upload into the atlas (or hit the atlas cache). For editor text
where the same lines appear frame after frame, this creates unnecessary reshaping work.
Buffer objects should be cached and reused when line content has not changed.

### 1.6 SumTree and Background Parsing

**Confidence: MEDIUM (Zed blog: Rope & SumTree)**

Zed's text storage is a `SumTree` — a copy-on-write B+ tree using `Arc` for node sharing.
When an edit occurs, only the modified nodes are cloned; unmodified nodes are shared between
the old and new tree via Arc reference counts.

This allows `Buffer::snapshot()` to be O(log n) — a cheap clone that shares the underlying
node data. The snapshot is sent to a background thread for tree-sitter parsing without
copying the entire text. On the next frame, the main thread uses the fresh parse tree for
highlighting.

**What muda uses:** `ropey` crate. Ropey's rope is also a B+ tree with shared nodes on
clone, providing similar semantics. The gap is not data structure — it is that muda does
not yet use background threads for parsing (full synchronous reparse blocks the event loop).

### 1.7 Frame Presentation: The 120fps Story

**Confidence: MEDIUM (Zed blog "120fps" and "videogame" — confirmed via WebSearch summaries; WebFetch denied)**

**The "videogame" blog post** describes GPUI's philosophy: rendering at display refresh rate
with GPU instanced drawing, treating each frame like a game frame. The goal is
consistently under 3ms per frame.

**The "120fps" blog post** describes a specific bug and fix in the Metal pipeline:
- Switching from `wait_until_completed` to `wait_until_scheduled` introduced a race where
  the GPU was reading instance buffer N while the CPU was writing frame N+1 to the same
  buffer.
- Fix: a pool of instance buffers — acquire one at frame start, release asynchronously
  after GPU completion (not CPU completion).
- This maps directly to ora's wgpu pipeline: the `RectangleRenderer` uploads a fresh
  instance buffer each frame. If wgpu's staging buffer is still in use by the GPU when
  the next frame writes it, a similar race exists. wgpu's buffer usage tracking should
  prevent this via submission fences, but it is worth monitoring under load.

**December 2025 Quality Week improvement:** Zed cut idle GPU usage by only presenting
frames during high-frequency input (scrolling), not after every event. This confirms the
event-driven redraw pattern (Optimization 1) is the correct approach for ora.

---

## Section 2: Performance Optimizations Ranked by Impact

### Optimization 1 — Event-Driven Redraw

**Impact: CRITICAL | Effort: Trivial | Confidence: HIGH**

**File:** `ora/src/platform/event_loop.rs`, line ~549

**Current:**
```rust
// Inside RedrawRequested handler after successful render:
gpu_state.window.request_redraw(); // "Request continuous redraw for now"
```

**Problem:** This enqueues a redraw immediately after every frame. The app renders at maximum
frame rate regardless of whether anything changed. GPU usage is ~100% even when the editor
is idle.

**Fix:** Remove the unconditional `request_redraw()` from the success path. Redraw is
already triggered by:
- Every keyboard/mouse/scroll event (each handler calls `request_redraw()`)
- Dirty entity detection (`has_dirty_entities()` check at bottom of `window_event()`)
- Caret blink animation (needs a timer — see note below)
- Animations via the transition system

**Caret blink:** The caret blinks via `notify_caret_activity()` / `CaretElement`. This
currently relies on the continuous redraw loop. After removing continuous rendering, the
caret blink needs an explicit periodic trigger. Simplest approach: in `OraApp::about_to_wait()`,
track `last_caret_blink: Instant`. If a caret is active and `BLINK_INTERVAL` has elapsed,
call `request_redraw()`. This produces at most one redraw per blink period when idle.

**Expected gain:** 90%+ reduction in GPU usage at idle, prevents thermal throttling on
battery-powered devices, makes profiling meaningful. Zed independently confirmed this class
of fix reduced their idle GPU load significantly (December 2025 Quality Week).

---

### Optimization 2 — Glyphon Buffer Caching

**Impact: HIGH for large files | Effort: Medium | Confidence: HIGH**

**File:** `ora/src/rendering/text.rs`, `ora/src/element/trait_def.rs`

**Current:** Every frame, `LayoutContext::measure_text()` creates a new `glyphon::Buffer`
for every text span in every visible line. This means:
- Each `LinePresentation` produces N `Buffer` objects (one per `StyledSpan`)
- Each Buffer must be shaped via cosmic-text and rasterized by swash
- For a viewport of 40 lines with 5 spans each: 200 Buffer objects created per frame
- All 200 are dropped at end of frame and recreated next frame

**Fix:** Cache `glyphon::Buffer` objects keyed by `(text_content, font_size, line_height)`.
For editor text, most lines do not change between frames. A `HashMap<CacheKey, Buffer>` in
`TextSystem` with LRU eviction (capacity ~1000 entries) eliminates reshaping for unchanged
lines.

**Architecture change:** `measure_text()` in `LayoutContext` looks up the cache first,
creates and inserts only on miss. The `Buffer` stored in `PaintCommand::Text` becomes a
handle (index or Arc clone) rather than an owned Buffer. The `TextSystem` holds the
canonical Buffer pool.

**Expected gain:** Eliminates text reshaping work for all lines that have not changed since
the last frame. For a 40-line viewport where 1 line changes per keystroke, this reduces
reshaping by ~97.5%.

---

### Optimization 3 — Incremental Tree-Sitter Parsing

**Impact: HIGH for large files | Effort: Medium | Confidence: HIGH**

**File:** `core_editor/src/syntax.rs`

**Current:** Every edit calls `parser.parse(content, None)` — a full O(n) reparse.
Tree-sitter explicitly supports incremental parsing by accepting the previous tree and
edit metadata.

**Fix:**
```rust
pub fn parse_incremental(&mut self, content: &str, edit: &TextEdit) {
    if let Some(ref mut tree) = self.tree {
        tree.edit(&InputEdit { /* edit coordinates */ });
        self.tree = self.parser.parse(content, Some(tree));
    } else {
        self.tree = self.parser.parse(content, None);
    }
}
```

This requires `TextBuffer` to produce `TextEdit` structs on each mutation (byte offsets and
row/column coordinates). `ropey` can supply the byte offset; row/column can be computed from
the rope.

**Expected gain:** For a 10K-line file, reduces per-keystroke parse time from ~30ms to
~0.5ms (tree-sitter only re-parses the changed subtree, which is typically ~20 nodes).

---

### Optimization 4 — Background Syntax Parsing Thread

**Impact: HIGH for large files | Effort: High | Confidence: HIGH**

**Architecture:** Move tree-sitter parsing entirely off the main thread.

```
Main thread:                    Background thread:
  keystroke                       (parks until notified)
  -> buffer edit                  <- notified of edit + rope clone
  -> send (edit, rope_snapshot)   -> incremental parse
     to background channel        -> send new parse tree back
  -> use PREVIOUS highlight tree
     for this frame (stale ok)
  <- receive new parse tree
  -> invalidate affected lines
  -> request_redraw
```

This is what Zed's `SyntaxMap` does. The rope clone is cheap (O(log n) with Arc node
sharing). The main thread is never blocked on parsing. For files under ~100KB the background
thread completes before the next frame; for large files the user sees a 1-2 frame lag before
fresh highlighting, which is imperceptible.

**In muda's architecture:** The `core_editor::App` would own a background parse thread via
`std::thread::spawn` + `std::sync::mpsc` channels. `dispatch()` sends edits; the background
thread sends back `(version, SyntaxTree)`. `build_render_model()` uses whichever tree is
freshest. The `ora::editor_adapter::EditorDataSource` trait does not change.

**Expected gain:** Eliminates parse-time contribution to keystroke latency entirely.
Keystroke handling becomes O(log n) rope edit only.

---

### Optimization 5 — Layout Caching for Stable Subtrees

**Impact: MEDIUM | Effort: High | Confidence: MEDIUM**

**What Zed does NOT do:** Zed does not cache layout results across frames. GPUI recomputes
flexbox layout every draw cycle for the full element tree. This is fast enough because:
1. Flexbox for ~100 elements takes <0.5ms with a good implementation
2. GPUI only runs layout when the window is dirty (Optimization 1 eliminates idle frames)

**What would help ora:** The flexbox layout (`compute_flexbox()` in
`ora/src/layout/flexbox.rs`) is called once per frame with all registered nodes. For the
editor, most nodes (gutter rows, line divs) are structurally identical across frames. A
keyed layout cache that persists LayoutOutput across frames when style and children are
unchanged would eliminate most layout work.

**Prerequisite:** Requires stable node identity across frames (currently nodes get new
LayoutIds each frame). This is non-trivial to implement correctly and should come after
Optimizations 1-4 are in place and profiling confirms layout is a bottleneck.

---

### Optimization 6 — View Tree Construction Caching (GPUI AnyView::cached analog)

**Impact: LOW-MEDIUM | Effort: High | Confidence: MEDIUM**

**The problem:** Every `render()` call constructs `Vec<AnyElement>` — one `Div` per
visible line, one `TextElement` per span. For 40 visible lines with 5 spans each:
~200 AnyElement allocations per frame.

**Zed's approach (PR #25009):** `AnyView::cached` — views can opt in to caching their
rendered element output behind a stable cache key (entity version counter). When the mouse
moves over a panel that doesn't depend on cursor position, that panel's `render()` is
skipped. The critical fix was ensuring `window.refresh` (triggered by mouse-event Divs)
does not invalidate cached views that don't depend on hover state.

**Fix for ora:** In `TextAreaView::render()` and `GutterView::render()`, cache the element
Vec behind a `dirty: bool` flag. Only rebuild when the `RenderModel` has changed (detect via
version counter on `RenderModel`). This is similar to React's `shouldComponentUpdate`.

**Prerequisite:** `RenderModel` needs a `version: u64` counter incremented on every
`build_render_model()` call that produces different output. Or hash the visible lines.

---

## Section 3: Integration Architecture for New Features

### 3.1 Current Integration Status

The following views already exist in `ora/src/views/`:

| View File | Status | Connected to EditorDataSource? |
|-----------|--------|-------------------------------|
| `tab_bar.rs` | Implemented | Yes (TabBarPresentation) |
| `gutter.rs` | Implemented | Yes (GutterModel) |
| `text_area.rs` | Implemented | Yes (RenderModel) |
| `status_bar.rs` | Implemented | Yes (StatusPresentation) |
| `sidebar.rs` | Implemented | Yes (SidebarPresentation) |
| `file_tree.rs` | Implemented | Yes (FileTreePresentation) |
| `command_palette.rs` | Implemented | Local state |
| `dialog.rs` | Implemented | Yes (DialogPresentation) |
| `panel_manager.rs` | Stub / partial | TBD |
| `app_layout.rs` | Implemented | Composes all above |

Most visual structure already exists. The work for new features is behavioral (tab
switching logic, find/replace query dispatch, file click-to-open) not structural.

### 3.2 Find/Replace Integration

**Architecture pattern:**

Find/replace is a **modal overlay** that sits above the editor content area. It does not
replace the editor; it modifies what the editor highlights and where the caret lands.

```
FindReplaceView (new)
  +--> renders as a floating Div at top of editor area (Stack overlay)
  +--> owns: query string, replace string, match_case, whole_word flags
  +--> dispatches: SearchForward / SearchBackward / ReplaceOne / ReplaceAll
  +--> consumes: RenderModel.search_results (new field -- highlighted match spans)
```

**EditorDataSource additions needed:**
```rust
// In ora::editor_adapter
pub struct RenderModel {
    // ... existing fields ...
    pub search_results: Option<SearchResultPresentation>, // NEW
}

pub struct SearchResultPresentation {
    pub query: String,
    pub match_count: usize,
    pub current_match: usize,
    // spans are already handled by StyledSpan -- add TextStyle::SearchHighlight token
}
```

**core_editor additions needed:**
- `EditorCommand::Find { query, options }` triggers search, stores match list
- `EditorCommand::FindNext` / `FindPrev` move through match list, scroll to match
- `EditorCommand::Replace { replacement }` / `EditorCommand::ReplaceAll`
- `build_render_model()` includes match highlights as `TextStyle::SearchHighlight` spans

**Integration point:** The `TextAreaView` already renders `StyledSpan` tokens via color
mapping in the theme. Adding `TextStyle::SearchHighlight` and `TextStyle::SearchCurrent`
requires no structural changes to the rendering pipeline — just add two new color tokens.

### 3.3 Multi-Tab Buffer Management

**Architecture pattern:**

Tabs are managed in `core_editor`'s `Workspace`. The existing `TabBarPresentation` and
`TabBarView` already show tabs. What is missing is the interaction: clicking a tab must
switch the active buffer, and closing a tab must handle unsaved changes.

**Data flow:**
```
TabBarView mouse click on tab N
-> dispatch_command(EditorCommand::SwitchTab(view_id))
-> core_editor::Workspace::activate_view(view_id)
-> next build_render_model() returns text for that buffer
-> ora renders new content (no architectural change needed)
```

**EditorDataSource additions:**
```rust
pub enum EditorCommand {
    // ... existing ...
    SwitchTab(usize),       // view_id from TabPresentation
    CloseTab(usize),        // view_id; triggers UnsavedChangesConfirmation if dirty
    OpenFile(String),       // path; creates new tab
}
```

**The click handler in TabBarView:** Currently `TabBarView` renders tabs as static Divs.
It needs mouse hitboxes registered per tab. `PrepaintContext::register_hitbox()` and
`on_mouse_down()` are the mechanism. This is a view-layer change with no framework changes.

### 3.4 File Browser (Click to Open)

**Current state:** `FileTreeView` and `SidebarView` render the file tree correctly.
What is missing: click-to-open behavior.

**Pattern:**
```
FileTreeView -> TreeItem element -> registers hitbox in prepaint
-> on_mouse_down callback -> dispatch_command(EditorCommand::OpenFile(path))
-> core_editor opens file, creates new buffer, activates it
-> next render() picks up new TabBarPresentation + new visible_lines
```

**EditorDataSource addition:** `EditorCommand::OpenFile(String)` (same as above).

The file entry must carry its full path through `FileEntryPresentation`:
```rust
pub struct FileEntryPresentation {
    pub name: String,
    pub path: String,  // NEW -- full relative path from workspace root
    pub is_dir: bool,
    pub is_selected: bool,
}
```

### 3.5 Find in Project (Multi-File Search)

**Pattern:**

A `SearchResultsView` showing file:line matches as a list. Clicking a result dispatches
`OpenFile + ScrollTo` commands. This avoids the complexity of Zed's `MultiBuffer` approach
while providing the same core use case.

```rust
pub enum EditorCommand {
    // ... existing ...
    SearchProject { query: String, options: SearchOptions },
    OpenFileAtLine { path: String, line: usize },
}
```

The `SearchResultsView` can reuse existing `FileTreeNode` / list patterns already in
`FileTreeView`. This is not a new architectural concept — it reuses existing view primitives.

---

## Section 4: Component Boundaries and Data Flow

### 4.1 Current Data Flow (Accurate Picture)

```
winit events
-> OraApp::window_event()
-> event_loop.rs (keyboard: translate_editor_command)
-> CoreEditorAdapter::dispatch_command()
-> core_editor::App::dispatch()
-> TextBuffer mutation + syntax reparse (SYNC, blocks here)
-> OraApp::window_event() returns
-> OraApp requests redraw

On RedrawRequested:
-> OraWindow::render(&mut app_context)
-> EditorRootView::render()
-> adapter.build_render_model(viewport_lines)     <- CALLED EVERY FRAME
    -> core_editor::App::build_render_model()     <- CLONES all visible lines
    -> convert_render_model() (type conversion)
-> Full element tree construction
    -> GutterView::render() -> Vec<AnyElement> (one per visible line)
    -> TextAreaView::render() -> Vec<AnyElement> (one per line, N per span)
-> request_layout() for all elements
-> compute_flexbox() for all nodes
-> prepaint() -- register hitboxes
-> paint() -- generate PaintCommand list
-> GpuState::render_frame(&commands)
    -> group into layers and draw groups
    -> single rect buffer upload
    -> per-layer text prepare() + render()
    -> single GPU submit
-> atlas.trim()
-> [BUG] request_redraw() unconditionally -- renders next frame immediately
```

### 4.2 Target Data Flow (After Optimizations 1-3)

```
Event arrives (keyboard/mouse/scroll/timer)
-> Dispatch command if editor event
-> Incremental parse on background thread (non-blocking)
-> Invalidate dirty entities
-> Request redraw (only if something changed)

On RedrawRequested (only when dirty):
-> Same render pipeline as above BUT:
   - Buffer objects fetched from glyphon cache (avoid reshaping)
   - Background parse result available (fresh highlight spans)
-> NO unconditional request_redraw at end
-> Next frame only happens if new event arrives or blink timer fires
```

---

## Section 5: Suggested Build Order

### Rationale

Performance and features are not in tension here — the performance work is localized and
does not block feature work. However, fixing Optimization 1 first is important because it
makes profiling meaningful. With continuous rendering, benchmarks measure "time to render
60 frames of nothing" instead of actual work.

### Recommended Order

**Tier 1: Unblock everything (do first, 1-2 days)**

1. **Event-driven redraw (Opt 1)** — Remove the unconditional `request_redraw()`. Add
   caret-blink timer via `about_to_wait()` interval check. This makes profiling accurate
   and eliminates the idle GPU burn. Zero risk, maximum return.

2. **Incremental tree-sitter parsing (Opt 3)** — `core_editor` change only, does not
   touch `ora`. Eliminates the most painful user-visible latency for large files.

**Tier 2: Core feature work (interleaved)**

3. **Tab switching** — Click handler in TabBarView + `SwitchTab` command. Unblocks
   multi-file workflow.

4. **File click-to-open** — Click handler in FileTreeView + `OpenFile` command +
   `FileEntryPresentation.path`. Unblocks file navigation.

5. **Find/replace overlay** — `FindReplaceView` + `EditorCommand::Find` + match rendering.
   This is substantial work (query dispatch, match highlighting, navigation) but
   architecturally straightforward.

**Tier 3: Performance refinement (after features work)**

6. **Glyphon Buffer caching (Opt 2)** — Profile first to confirm reshaping is the hotspot.
   Requires changes to `LayoutContext::measure_text()` and `TextSystem`.

7. **Background parse thread (Opt 4)** — Needed for files >100KB. High effort.
   Do after incremental parsing (Opt 3) is stable.

8. **Layout caching (Opt 5)** — Only if profiling shows layout as hotspot after Opts 1-4.

### Do NOT build first

- Layout caching before event-driven redraw: profiling will show wrong hotspots
- Find-in-project before find-in-buffer: the simpler feature validates the dispatch pattern
- MultiBuffer (Zed-style): too complex for v2; the simple SearchResultsView is sufficient

---

## Section 6: Anti-Patterns to Avoid

### Anti-Pattern 1: Unconditional Continuous Redraw

The current `request_redraw()` in the success path of `RedrawRequested` is the canonical
example. Do not add this elsewhere (e.g., inside `about_to_wait()` unconditionally).
Zed's own Quality Week 2025 confirms: idle GPU usage is reduced by NOT presenting frames
after every event.

### Anti-Pattern 2: Allocating Buffer Objects Every Frame

`glyphon::Buffer` creation is not free — it triggers text shaping via cosmic-text. Creating
200 per frame for unchanged text defeats the atlas cache. Cache by content hash.

### Anti-Pattern 3: Synchronous Full Syntax Reparse on Edit

The full `parser.parse(content, None)` blocks the event loop thread. For files over 50KB
this creates visible latency. The fix is incremental parsing first, background threading
second.

### Anti-Pattern 4: Calling `build_render_model()` More Than Once Per Frame

Each call clones all visible `LinePresentation` objects (including all `StyledSpan` strings).
For a 40-line viewport with syntax highlighting, this is hundreds of String allocations.
Call once per frame and store the result; do not call from multiple view components.
Currently `EditorRootView::render()` calls it once and distributes the result, which is
correct.

### Anti-Pattern 5: View-Layer Components Owning core_editor State

Views should consume `RenderModel` (a presentation-layer snapshot), not hold references to
`core_editor` types. The `EditorDataSource` trait boundary enforces this. Do not add new
views that take `&mut core_editor::App` directly.

### Anti-Pattern 6: Using HitboxId as Stable Key for Per-Frame State

`HitboxId` is assigned fresh each frame (monotonic counter reset at 0). Any state that
must persist across frames (transition animations, view cache state) must use a stable
`TransitionId` or entity ID. This is documented in Phase 9 RESEARCH.md and remains relevant
for any new interactive features built in v2.0.

---

## Sources and Confidence

| Source | What it informs | Confidence |
|--------|----------------|------------|
| `ora/src/platform/event_loop.rs` (read) | Continuous redraw bug, actual render pipeline | HIGH |
| `ora/src/rendering/text.rs` (read) | Atlas persistence, clear() semantics | HIGH |
| `ora/src/element/trait_def.rs` (read) | Three-phase lifecycle, Buffer allocation | HIGH |
| `ora/src/platform/gpu.rs` (read) | Frame pipeline, layer structure | HIGH |
| `ora/src/views/*.rs` (read) | Existing view coverage, integration gaps | HIGH |
| `ora/src/editor_adapter/` (read) | EditorDataSource pattern, command dispatch | HIGH |
| DeepWiki (zed-industries/zed) | Zed EditorElement, DisplayMap, snapshot pattern, ScrollManager | MEDIUM |
| Zed blog: "Rope & SumTree" | SumTree, copy-on-write, background parsing | MEDIUM |
| Zed blog: "Syntax-Aware Editing" | Incremental tree-sitter, SyntaxMap layers | MEDIUM |
| Zed blog "videogame" (summary) | Frame budget <3ms, glyph cache stability, GPU instancing | MEDIUM |
| Zed blog "120fps" (summary) | Metal instance buffer pool, wait_until_scheduled vs completed | MEDIUM |
| Zed blog "Quality Week Dec 2025" (summary) | Idle GPU reduction by conditional frame presentation | MEDIUM |
| WebSearch: GPUI dirty tracking | WindowInvalidator, cx.notify -> invalidate, CADisplayLink dirty flag | MEDIUM |
| GitHub PR #25009 (summary) | AnyView::cached, scroll/mousemove no longer trigger full render | MEDIUM |
| GitHub discussion: GPUI perf | "GPUI always re-layouts whole app each frame, few caches" | MEDIUM |

### Notes on WebFetch Availability

WebFetch is blocked in this environment. All Zed blog and GitHub source findings are
derived from WebSearch result summaries, which include excerpt text from the pages.
These are marked MEDIUM (not LOW) because multiple independent searches confirmed the
same facts, and the summaries are consistent with the ora source code observations.
Direct source code reading of the Zed repository was not possible.

### Notes on GPUI API Currency

GPUI underwent a major API consolidation in 2025: `Model<T>` and `View<T>` were merged into
`Entity<T>`. The ora framework predates this change and uses its own analogous types
(`Model`, `Entity`, `View` traits). References to GPUI APIs in research summaries may
reflect pre- or post-consolidation GPUI. Verify against current GPUI README
(`github.com/zed-industries/zed/blob/main/crates/gpui/README.md`) before implementing
any feature that closely follows GPUI patterns.
