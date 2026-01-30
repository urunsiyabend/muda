---
phase: 07
plan: 02
subsystem: views
tags: [sidebar, gutter, editor-chrome, collapsible-panel, line-numbers]
dependency-graph:
  requires: [06-design-system]
  provides: [SidebarView, GutterView]
  affects: [08-file-tree, TextAreaView]
tech-stack:
  added: []
  patterns: [collapsible-panel, dynamic-width-calculation]
key-files:
  created:
    - ora/src/views/sidebar.rs
    - ora/src/views/gutter.rs
  modified:
    - ora/src/views/mod.rs
    - ora/src/lib.rs
    - ora/src/views/tab_bar.rs
decisions:
  - key: sidebar-collapsed-state
    choice: 48px icon rail (not completely hidden)
    reason: CONTEXT decision - provides expand affordance
  - key: sidebar-toggle-method
    choice: dedicated button (not header click)
    reason: CONTEXT decision - explicit toggle control
  - key: gutter-current-line
    choice: text color only (not background)
    reason: CONTEXT decision - parent layout handles highlight background
metrics:
  duration: ~8m
  completed: 2026-01-30
---

# Phase 07 Plan 02: Sidebar and Gutter Views Summary

SidebarView and GutterView implemented as ora Views for vertical edge chrome.

## What Was Built

### SidebarView (314 lines)
Collapsible file explorer panel with:
- Expanded state (default 220px): Header with "Explorer" title, ghost toggle button, content placeholder
- Collapsed state (48px): Narrow icon rail with "Files" expand button
- Width clamping (100-400px) with set_width()
- Consumes SidebarPresentation from core_editor

### GutterView (301 lines)
Line number gutter with:
- Dynamic width calculation: LEFT_PADDING + (digits * char_width) + SEPARATOR_PADDING
- Current line highlighting with FgPrimary color
- Other lines with FgMuted for visual hierarchy
- Right-aligned line numbers with separator column
- Consumes GutterModel and visible_lines from core_editor

## Key Implementation Details

**Borrow Checker Pattern**
Both views use the same pattern to avoid borrowing issues:
1. Build child elements first (which borrow cx mutably)
2. Get theme reference after child building
3. Use theme for container styling

**Width Calculation (GutterView)**
```rust
pub fn calculate_width(gutter: &GutterModel, char_width: f32) -> f32 {
    LEFT_PADDING + (digit_count as f32 * char_width) + SEPARATOR_PADDING
}
```
- 9 lines: 32px (1 digit)
- 99 lines: 40px (2 digits)
- 999 lines: 48px (3 digits)
- 9999 lines: 56px (4 digits)

## Commits

| Hash | Description |
|------|-------------|
| 6fa220b | feat(07-02): implement SidebarView for collapsible file explorer panel |
| e0cacaa | feat(07-02): implement GutterView for line number display |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed borrow checker in TabBarView**
- Found during: Task 1 verification
- Issue: tab_bar.rs had borrow checker error with theme and cx
- Fix: Moved theme lookup after map() closure
- Files modified: ora/src/views/tab_bar.rs
- Commit: 6fa220b

**2. [Rule 3 - Blocking] Fixed missing ColorToken::Warning mapping**
- Found during: Task 2 verification
- Issue: Warning token existed but match was incomplete
- Fix: Added amber_500()/amber_600() to theme match
- Files modified: ora/src/theme/mod.rs (already fixed by linter)

**3. [Rule 1 - Bug] Fixed ColorToken::BorderDefault typo**
- Found during: Task 2 compilation
- Issue: Used non-existent BorderDefault instead of Border
- Fix: Changed to ColorToken::Border in gutter.rs
- Commit: e0cacaa

## Tests Added

SidebarView tests (5):
- test_sidebar_view_creation
- test_sidebar_toggle
- test_sidebar_width_calculation
- test_sidebar_width_clamping
- test_sidebar_constants

GutterView tests (6):
- test_gutter_view_creation
- test_calculate_width_visible
- test_calculate_width_not_visible
- test_calculate_width_different_line_counts
- test_gutter_view_width_method
- test_gutter_constants

Total ora tests: 33 (all passing)

## Verification Results

1. cargo check -p ora - Compiles with expected warnings
2. cargo test -p ora - 33 tests pass
3. SidebarView shows icon rail (48px) when collapsed
4. SidebarView shows full panel with header when expanded
5. GutterView calculates width based on line count
6. GutterView highlights current line number

## Next Phase Readiness

Ready for Plan 03 (TextAreaView and CaretView):
- Theme color tokens established
- View pattern well-established
- SidebarView provides left edge layout reference
- GutterView provides line-aligned rendering pattern
