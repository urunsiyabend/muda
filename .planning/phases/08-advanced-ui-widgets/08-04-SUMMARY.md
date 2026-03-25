---
phase: 08
plan: 04
name: command-palette
subsystem: views
tags: [rust, command-palette, fuzzy-search, overlay, keyboard-navigation]

dependency-graph:
  requires:
    - "08-01 (Input element for search field concept)"
    - "08-02 (ListItem pattern for result rows)"
    - "06-design-system (ColorToken, Theme)"
  provides:
    - "CommandPaletteView — VS Code-style searchable command overlay"
    - "fuzzy_match() — fuzzy matching algorithm with scoring and match positions"
    - "CommandItem — command entry type with label and shortcut"
    - "FuzzyMatch — match result with score and character positions"
  affects:
    - "08-08 (AppLayout integrates CommandPalette as overlay)"

tech-stack:
  added: []
  patterns:
    - "Fuzzy match scoring: +10 consecutive, +5 word boundary, +1 base"
    - "Character-level highlight segments: split label into matched/unmatched TextElements"
    - "Overlay positioning: full-screen flex_col + align_center + pt offset"
    - "Filtered result list with score-descending sort"

key-files:
  created:
    - "ora/src/views/command_palette.rs — CommandPaletteView, fuzzy_match, CommandItem, FuzzyMatch"
  modified:
    - "ora/src/views/mod.rs — registered command_palette module"
    - "ora/src/lib.rs — added CommandPaletteView re-exports"
---

## What was built

**CommandPaletteView** — A VS Code-style command palette overlay with fuzzy search filtering.

### Components

1. **fuzzy_match()** — Case-insensitive fuzzy matching with scoring. Consecutive character matches score +10, word boundary matches score +5, basic matches score +1. Returns match positions for character-level highlighting.

2. **CommandItem** — Command entry with `id`, `label`, and optional `shortcut` display text. Builder pattern with `with_shortcut()`.

3. **CommandPaletteView** — View rendering a centered overlay (600px wide, 80px from top) with:
   - Search input row showing "> " prefix and query text
   - Separator line
   - Up to 8 filtered results with score-sorted ordering
   - Selected result highlighted with BgSecondary background
   - Matched characters highlighted in Accent color
   - Keyboard navigation (up/down/enter) via move_selection_up/down/selected_command

### Commits

| Hash | Message |
|------|---------|
| 488149a | feat(08-04): add CommandPaletteView with fuzzy search |

### Test Results

- 7 fuzzy_match unit tests (empty query, exact match, non-contiguous, no match, case insensitive, consecutive bonus, word boundary bonus)
- 2 CommandPaletteView tests (creation, filtering)
- All 106 lib tests pass

### Decisions

- Fuzzy match is a pure function (no struct) for simplicity and testability
- render_highlighted_label splits label into TextElement segments with Accent/FgPrimary colors
- No backdrop for palette (it's an overlay, not a modal like Dialog)
- PALETTE_WIDTH=600px, VISIBLE_ROWS=8, INPUT_HEIGHT=36px
