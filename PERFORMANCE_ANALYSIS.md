# Muda Editor Performance Analysis

## Executive Summary

This document provides a comprehensive performance analysis of the Muda text editor, identifying critical hot paths, bottlenecks, and proposing an optimization roadmap to achieve performance comparable to modern editors like Zed and Helix.

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│ Event Loop (main.rs)                                              │
│   └─> event::read() [BLOCKING]                                   │
│       └─> build_render_model() [EVERY FRAME]                     │
│           └─> ViewModelBuilder::build()                          │
│               ├─> document.content() [FULL CLONE]                │
│               ├─> build_visible_lines() [PER-LINE WORK]          │
│               │   └─> highlight_line() [TREE-SITTER QUERY]       │
│               └─> cursor/status calculations                     │
│       └─> terminal.draw() [RATATUI RENDER]                       │
└──────────────────────────────────────────────────────────────────┘
```

---

## Hot Path Analysis

### 1. Per-Keypress Edit Flow

When a user presses a character key, the following operations execute:

| Step | File:Line | Operation | Complexity | Notes |
|------|-----------|-----------|------------|-------|
| 1 | main.rs:294-298 | Key event processing | O(1) | Fast |
| 2 | dispatcher.rs:376-387 | `handle_insert_char()` | O(1) | Creates EditOperation |
| 3 | text_buffer.rs:200-205 | `rope.insert_char()` | O(log n) | Rope edit - FAST |
| 4 | syntax.rs:111-128 | `parse()` - FULL REPARSE | **O(n)** | **CRITICAL BOTTLENECK** |
| 5 | dispatcher.rs:168-174 | Event emission | O(1) | |
| 6 | app.rs:75 | `build_render_model()` | O(visible_lines) | Every frame |
| 7 | builder.rs:201 | `document.content()` | **O(n)** | **FULL STRING CLONE** |
| 8 | builder.rs:296 | `highlight_line()` per line | O(captures) | Per visible line |
| 9 | draw.rs:71-103 | Terminal render | O(visible_area) | |

**Total worst-case per keystroke: O(n) where n = file size**

### 2. Per-Keypress Navigation Flow

Cursor movement is lighter but still has issues:

| Step | File:Line | Operation | Complexity |
|------|-----------|-----------|------------|
| 1 | main.rs:209-246 | Key processing | O(1) |
| 2 | dispatcher.rs:206-247 | `handle_move_cursor()` | O(log n) for position conversion |
| 3 | main.rs:75 | `build_render_model()` | O(visible_lines) |
| 4 | builder.rs:201 | `document.content()` | **O(n) - UNNECESSARY** |

### 3. Render Frame Flow

Every frame rebuilds the entire view model:

```rust
// main.rs:75 - Called EVERY frame, even for cursor blink
let render_model = app.build_render_model(viewport_height);
```

No dirty checking. No incremental updates.

---

## Critical Bottlenecks (Prioritized)

### P0: Full Syntax Reparse on Every Edit
**Location:** `syntax.rs:111-128`
**Impact:** O(n) on every keystroke

```rust
pub fn parse(&mut self, content: &str) {
    // Note: Tree-sitter supports incremental parsing by passing the old tree,
    // but that requires calling tree.edit() with edit info before re-parsing.
    // For now, we do a full reparse which is simpler and correct.
    let new_tree = self.parser.parse(content, None);  // <-- Always full reparse
    self.tree = new_tree;
}
```

**Why it's expensive:**
- For a 10KB file (~300 lines): ~1-3ms per parse
- For a 100KB file (~3000 lines): ~10-30ms per parse
- For a 1MB file (~30000 lines): ~100-300ms per parse

**Tree-sitter DOES support incremental parsing** - the comment acknowledges it but doesn't implement it.

### P0: Full Content Clone Every Frame
**Location:** `view_model/builder.rs:201`

```rust
fn build_visible_lines(...) -> Vec<LinePresentation> {
    let content = document.content();  // <-- Returns String, full clone
    // ...
}
```

This clones the entire document text on every frame, even when only rendering 40 visible lines.

### P1: Per-Line String Allocations
**Location:** `view_model/builder.rs:220-224`

```rust
let display_text: String = line_text
    .chars()
    .skip(scroll_x)
    .take(width)
    .collect();  // <-- New String per visible line per frame
```

And in `build_line_spans()` (lines 328, 364):
```rust
let chars: Vec<char> = text.chars().collect();  // <-- Vec allocation
```

### P1: New QueryCursor Per Line
**Location:** `syntax.rs:177`

```rust
pub fn highlight_line(...) -> Vec<HighlightSpan> {
    let mut cursor = QueryCursor::new();  // <-- New cursor per line
    cursor.set_byte_range(line_start_byte..line_end_byte);
    // ...
}
```

QueryCursor allocation should be reused across lines.

### P1: No Dirty Tracking for Render
**Location:** `main.rs:70-76`

```rust
while !app.should_quit {
    // ALWAYS builds render model, even if nothing changed
    let render_model = app.build_render_model(viewport_height);
    terminal.draw(|f| ui(f, &render_model))?;
    // ...
}
```

### P2: panic::catch_unwind Overhead
**Location:** `syntax.rs:181-212`

```rust
let captures_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    // ... highlighting logic
}));
```

`catch_unwind` has non-trivial overhead. Should use proper bounds checking instead.

### P2: TextBuffer::line() String Allocation
**Location:** `text_buffer.rs:119-133`

```rust
pub fn line(&self, line_idx: usize) -> String {
    let line = self.rope.line(line_idx);
    let mut s = line.to_string();  // <-- Allocation
    // ... trim newlines
    s
}
```

Returns owned String instead of rope slice.

---

## Comparison with Zed/Helix

| Feature | Muda (Current) | Zed | Helix |
|---------|----------------|-----|-------|
| **Text Buffer** | Rope (ropey) | Rope (custom) | Rope (ropey) |
| **Syntax Parsing** | Full reparse every edit | Incremental tree-sitter | Incremental tree-sitter |
| **Highlight Caching** | None | Per-line cache | Per-line cache with invalidation |
| **Viewport Rendering** | Full rebuild every frame | Incremental diff | Incremental with damage tracking |
| **Memory Allocation** | Many per-frame allocs | Arena allocators | Minimal per-frame allocs |
| **Async Parsing** | Sync (blocks UI) | Background thread | Background thread |
| **Large File (1MB)** | ~100-300ms/keystroke | <16ms/keystroke | <16ms/keystroke |

### Key Gaps

1. **Incremental Parsing**: Zed/Helix use tree-sitter's incremental parsing. Muda does full reparse.
2. **Background Parsing**: Modern editors parse on background threads. Muda blocks the main thread.
3. **View Caching**: No caching of rendered lines or syntax highlights.
4. **Memory Strategy**: Heavy allocation patterns vs arena/pooling in modern editors.

---

## Optimization Roadmap

### Phase 1: Critical Path Fixes (Expected: Major improvements)

#### 1.1 Implement Incremental Tree-Sitter Parsing
- Call `tree.edit()` with edit info before reparsing
- Pass old tree to `parser.parse(content, Some(&old_tree))`
- **Expected improvement:** 10-100x for large files

#### 1.2 Eliminate Full Content Clone
- Change `document.content()` to return `&str` or rope reference
- Use rope slicing for visible lines only
- **Expected improvement:** 2-5x memory reduction

#### 1.3 Add Dirty Tracking
- Track what changed (cursor, content, selection, viewport)
- Only rebuild affected parts of render model
- **Expected improvement:** 50%+ reduction in work for navigation

### Phase 2: Rendering Optimization

#### 2.1 Cache Highlighted Lines
- Store `Vec<HighlightSpan>` per line with revision number
- Invalidate only affected lines on edit
- **Expected improvement:** Eliminate redundant highlighting

#### 2.2 Reuse QueryCursor
- Pool or reuse QueryCursor instances
- One cursor per frame, reset byte range per line

#### 2.3 Use String Slices
- Replace `String` with `&str` where possible
- Use rope `Chars` iterator instead of collecting to Vec

### Phase 3: Async & Background Processing

#### 3.1 Background Syntax Parsing
- Move tree-sitter parsing to dedicated thread
- Use channels to communicate parse results
- Parse asynchronously, highlight from last valid tree

#### 3.2 Debounced Parsing
- Don't reparse on every keystroke
- Wait for typing pause (100-200ms) before full parse
- Use previous tree for immediate highlighting

### Phase 4: Advanced Optimizations

#### 4.1 Virtual Viewport
- Only process lines within viewport + buffer zone
- Lazy load line content on scroll

#### 4.2 Arena Allocation
- Use arena allocator for per-frame data
- Reduce GC pressure and fragmentation

#### 4.3 GPU Rendering (Optional)
- Consider wgpu backend for very large files
- Hardware-accelerated text rendering

---

## Benchmark Targets

| Metric | Current (est.) | Target | Zed/Helix |
|--------|----------------|--------|-----------|
| Keystroke latency (1KB file) | 1-5ms | <1ms | <1ms |
| Keystroke latency (100KB file) | 20-50ms | <5ms | <2ms |
| Keystroke latency (1MB file) | 100-300ms | <16ms | <10ms |
| Navigation latency | 5-20ms | <1ms | <1ms |
| Render frame time | 5-15ms | <5ms | <3ms |
| Memory per MB of text | ~5-10MB | <3MB | <2MB |

---

## Files Requiring Changes

| Priority | File | Changes Needed |
|----------|------|----------------|
| P0 | `syntax.rs` | Incremental parsing, cursor reuse |
| P0 | `domain/document.rs` | Add content reference, track dirty state |
| P0 | `view_model/builder.rs` | Avoid full content clone, cache lines |
| P1 | `main.rs` | Add dirty checking before render |
| P1 | `text_buffer.rs` | Return slices instead of Strings |
| P2 | `app.rs` | Track dirty flags, debounce parsing |
| P2 | `commands/dispatcher.rs` | Emit more granular change events |

---

## Implementation Notes

### Incremental Tree-Sitter Parsing

```rust
// Required changes to SyntaxHighlighter
pub fn parse_incremental(&mut self, content: &str, edit: &TextEdit) {
    if let Some(ref mut tree) = self.tree {
        // Apply edit information to old tree
        tree.edit(&InputEdit {
            start_byte: edit.start_byte,
            old_end_byte: edit.old_end_byte,
            new_end_byte: edit.new_end_byte,
            start_position: edit.start_position,
            old_end_position: edit.old_end_position,
            new_end_position: edit.new_end_position,
        });
        // Incremental reparse using old tree
        self.tree = self.parser.parse(content, Some(tree));
    } else {
        self.tree = self.parser.parse(content, None);
    }
}
```

### Dirty Tracking

```rust
#[derive(Default)]
struct DirtyFlags {
    content: bool,
    cursor: bool,
    selection: bool,
    viewport: bool,
}

impl App {
    fn build_render_model_incremental(&mut self) -> RenderModel {
        if !self.dirty.content && !self.dirty.viewport {
            // Only update cursor position in existing model
            return self.cached_model.with_updated_cursor(...);
        }
        // Full rebuild only when needed
        self.rebuild_full_model()
    }
}
```

---

## Conclusion

The Muda editor has a clean architecture but critical performance issues in its hot paths:

1. **O(n) syntax reparsing** on every keystroke is the primary bottleneck
2. **Full content cloning** per frame wastes memory and CPU
3. **No caching or dirty tracking** causes redundant work

Implementing incremental parsing and dirty tracking would bring Muda to competitive performance levels with Helix for typical file sizes (<100KB). For 1MB+ files, background parsing would also be necessary.

The benchmarking suite in `benches/` provides tooling to measure progress against these targets.
