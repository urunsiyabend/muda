---
status: testing
phase: 08-advanced-ui-widgets
source: 08-01-SUMMARY.md, 08-02-SUMMARY.md, 08-03-SUMMARY.md, 08-04-SUMMARY.md, 08-05-SUMMARY.md, 08-06-SUMMARY.md, 08-07-SUMMARY.md, 08-08-SUMMARY.md
started: 2026-03-02T09:10:00Z
updated: 2026-03-02T09:10:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 1
name: Unit Tests Pass
expected: |
  Run `cargo test -p ora --lib` — all 110 tests pass with no failures or compilation errors.
awaiting: user response

## Tests

### 1. Unit Tests Pass
expected: Run `cargo test -p ora --lib` — all 110 tests pass with no failures or compilation errors.
result: [pending]

### 2. Demo Window Launches
expected: Run `cargo run --example phase8_demo` — a 1400x900 window opens titled "ora - Phase 8 Integration Demo" with dark theme background. No panics or GPU errors in console.
result: [pending]

### 3. IDE Layout Structure
expected: Window shows a full IDE layout: collapsible sidebar on the left, horizontal tab bar across the top of the editor area, line-number gutter, syntax-highlighted code area in the center, status bar at the very bottom, and a bottom panel between the editor and status bar.
result: [pending]

### 4. File Tree in Sidebar
expected: Left sidebar shows a file tree with "my_project" as root. Expanded folders "src" (showing main.rs, lib.rs, collapsed views/ and elements/) are visible. Folders show a different icon color (amber) than files. File names are readable.
result: [pending]

### 5. Tab Bar with Tabs
expected: Tab bar shows 3 file tabs: "main.rs" (active, with accent bottom border or highlight), "*app_layout.rs" (with dirty/unsaved indicator), and "lib.rs". Tabs have close buttons.
result: [pending]

### 6. Syntax-Highlighted Code
expected: Editor area shows Rust code with syntax coloring — keywords ("use", "fn", "let") in one color, types ("AppLayout", "CommandPaletteView") in another, strings in a distinct color, comments grayed out. Line numbers visible in the gutter. Line 5 has current-line highlight.
result: [pending]

### 7. Command Palette Overlay
expected: A command palette overlay appears centered near the top of the window. It shows "> open" in the search input. Below it, filtered results matching "open" are listed (e.g., "Open File Ctrl+O", "Open Settings Ctrl+,"). One result is highlighted/selected.
result: [pending]

### 8. Panel Manager
expected: A bottom panel is visible below the editor area. It has tab labels for different panel types (Output, Problems, Terminal, Debug). The active tab has accent styling. Content area shows placeholder text.
result: [pending]

### 9. Status Bar
expected: Bottom status bar displays: file name context, cursor position (Ln 5, Col 1 or similar), language "Rust", and the message "Phase 8 complete". Text is readable against the bar background.
result: [pending]

### 10. Toast Notification
expected: A toast notification appears at the bottom-right corner of the window showing "Phase 8 complete — all widgets integrated" with a green/success-colored accent strip on the left side.
result: [pending]

### 11. Theme Toggle
expected: Press the 'T' key — the entire UI switches between dark and light theme. All regions (sidebar, editor, tabs, status bar, panels, overlays) update their colors consistently. Press 'T' again to switch back.
result: [pending]

## Summary

total: 11
passed: 0
issues: 0
pending: 11
skipped: 0

## Gaps

[none yet]
