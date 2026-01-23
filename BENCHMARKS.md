# Muda Editor Benchmarks

## Quick Start

Run all benchmarks:
```bash
cargo bench
```

Run specific benchmark suite:
```bash
cargo bench --bench text_buffer_benchmarks
cargo bench --bench syntax_benchmarks
cargo bench --bench render_benchmarks
cargo bench --bench editor_benchmarks
```

Run with reduced sample size (faster, less accurate):
```bash
cargo bench -- --sample-size 10
```

Filter to specific benchmarks:
```bash
cargo bench -- "text_buffer/insert_char"
cargo bench -- "syntax/full_parse"
cargo bench -- "editor/keypress"
```

## Benchmark Suites

### 1. Text Buffer Benchmarks (`text_buffer_benchmarks.rs`)

Measures core text buffer operations using the Rope data structure:

| Benchmark | Description |
|-----------|-------------|
| `insert_char/beginning` | Single char insert at document start |
| `insert_char/middle` | Single char insert at document middle |
| `insert_char/end` | Single char insert at document end |
| `insert_string` | Multi-character string insertion |
| `delete/single_char` | Delete single character |
| `delete/100_chars` | Delete 100 characters |
| `delete/line` | Delete entire line |
| `coordinates/offset_to_position` | Convert offset to (line, column) |
| `coordinates/position_to_offset` | Convert (line, column) to offset |
| `line_access/get_line` | Get single line content |
| `line_access/get_40_lines` | Get viewport-sized batch of lines |
| `full_content/to_string` | Clone entire document to String |
| `typing_throughput` | Rapid character insertion |

### 2. Syntax Benchmarks (`syntax_benchmarks.rs`)

Measures tree-sitter based syntax highlighting:

| Benchmark | Description |
|-----------|-------------|
| `full_parse/rust` | Parse Rust source file |
| `full_parse/json` | Parse JSON file |
| `full_parse/plain` | Parse plain text (no highlighting) |
| `highlight_line` | Highlight single line |
| `highlight_viewport/40_lines` | Highlight 40-line viewport |
| `edit_reparse/char_insert` | Insert char + full reparse (current bottleneck) |
| `highlighter_creation` | Create new highlighter instance |

### 3. Render Benchmarks (`render_benchmarks.rs`)

Measures view model building and rendering pipeline:

| Benchmark | Description |
|-----------|-------------|
| `build_model/full_model` | Complete RenderModel construction |
| `cursor_positions` | Model build at different cursor positions |
| `selection/no_selection` | Model build without selection |
| `selection/small_selection` | Model build with 10-char selection |
| `selection/select_all` | Model build with full selection |
| `viewport_size` | Different viewport sizes (80x24, 120x40, etc.) |
| `sidebar_toggle` | Impact of sidebar visibility |
| `content_access/full_clone` | Clone entire document (current approach) |
| `content_access/40_lines_only` | Get only visible lines (optimized) |

### 4. Editor Benchmarks (`editor_benchmarks.rs`)

End-to-end editor operations:

| Benchmark | Description |
|-----------|-------------|
| `keypress_edit/insert_char` | Complete edit cycle (command + render) |
| `keypress_navigation/arrow_right` | Horizontal cursor movement |
| `keypress_navigation/arrow_down` | Vertical cursor movement |
| `keypress_navigation/page_down` | Page navigation |
| `typing_throughput/chars` | Insert N chars rapidly |
| `typing_with_render/chars_per_frame` | Insert chars with per-key render |
| `undo_redo/undo_10` | Undo 10 operations |
| `selection_operations` | Select/delete operations |
| `large_file/open_1MB` | Open large file |
| `large_file/first_keypress` | First edit in 1MB file |

## Saving Baselines

Save current performance as baseline:
```bash
cargo bench -- --save-baseline before_optimization
```

Compare against baseline after changes:
```bash
cargo bench -- --baseline before_optimization
```

## Viewing Results

HTML reports are generated in `target/criterion/`.
Open `target/criterion/report/index.html` in a browser.

## Performance Targets

| Metric | Current (est.) | Target | Notes |
|--------|----------------|--------|-------|
| Keystroke (10KB) | 5-10ms | <1ms | Includes full reparse |
| Keystroke (100KB) | 20-50ms | <5ms | |
| Keystroke (1MB) | 100-300ms | <16ms | Must be under 60fps |
| Navigation | 1-5ms | <1ms | No reparsing needed |
| Render frame | 5-15ms | <5ms | 40-line viewport |

## Identified Bottlenecks

See `PERFORMANCE_ANALYSIS.md` for detailed analysis.

1. **P0: Full Syntax Reparse** - O(n) on every keystroke
2. **P0: Full Content Clone** - Clones entire document every frame
3. **P1: No Dirty Tracking** - Always rebuilds full render model
4. **P1: Per-Line Allocations** - String allocations in hot path
5. **P2: QueryCursor Creation** - New cursor per highlighted line
