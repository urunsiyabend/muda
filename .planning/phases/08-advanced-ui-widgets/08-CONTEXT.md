# Phase 8: Advanced UI & Widgets — Context

## Command Palette

### Activation
- **Ctrl+Shift+P only** — single keybinding, no separate quick-open mode
- No Ctrl+P file picker variant in this phase

### Search Behavior
- **Fuzzy matching** — matches non-contiguous characters (e.g., "ofi" matches "Open File")
- **Highlight matched characters** — render matched chars in bold/accent color within result text

### Result Display
- **Fixed visible count (8-10 results)** — palette is compact, scroll for more
- Overlay centered horizontally near top of viewport (VS Code-style positioning)

---

## File Tree

### Expand/Collapse
- **Arrow click OR double-click** to expand/collapse folders
- Single click on label selects without expanding
- Arrow keys for keyboard navigation

### Selection
- **Multi-select supported** — Ctrl+click for individual, Shift+click for range
- Enables future batch operations (move, delete, etc.)

### Visual Treatment
- **Subtle indent guides** — thin vertical lines at each indent level
- **Colored file-type icons** — different icons and colors per file extension (Seti/Material style)

---

## Widget Toolkit

### Size Tiers
- **3 tiers: sm, md, lg**
  - `sm` — compact areas (tree items, tabs, status bar)
  - `md` — standard controls (buttons, inputs, checkboxes)
  - `lg` — prominent actions (dialog buttons, main CTAs)

### Interaction Feedback
- **Background highlight** — subtle background color shift on hover, darker shade on active/pressed
- No border changes on hover (keep it clean)

### Disabled State
- **Dimmed + no cursor** — reduced opacity (~50%) and `cursor: default`
- Clearly communicates non-interactive state

### Toast Notifications
- **Severity-based behavior:**
  - Info/success → auto-dismiss after 3-5 seconds
  - Warning/error → persist until user dismisses
- Position: bottom-right or top-right (Claude decides based on layout)

---

## Layout & Panels

### Panel Resize
- **Drag handle + presets** — drag border between editor and bottom panel
- Double-click handle to toggle between collapsed / half / full presets

### Persistence
- **Panel sizes persist across sessions** — save heights/widths, restore on launch

### Region Toggle
- **Instant toggle** — no animation when showing/hiding sidebar or bottom panel
- Snappy, immediate layout recalculation

### Panel Tabs
- **Drag to reorder** — users can drag tabs (Output, Problems, etc.) to customize order
- Tab order persists with panel size preferences

---

## Deferred Ideas
_(Captured during discussion but out of scope for Phase 8)_

- None identified

---

## Decisions Summary

| Area | Decision | Rationale |
|------|----------|-----------|
| Palette activation | Ctrl+Shift+P only | Single standard keybinding |
| Palette search | Fuzzy match + highlight | VS Code-familiar UX |
| Palette results | Fixed 8-10 visible | Compact overlay |
| Tree expand | Arrow or double-click | Separates select from expand |
| Tree selection | Multi-select (Ctrl/Shift) | Enables batch operations |
| Tree visuals | Indent guides + colored icons | Professional file browser feel |
| Widget sizes | 3 tiers (sm/md/lg) | Covers all density contexts |
| Widget hover | Background highlight only | Clean, minimal feedback |
| Widget disabled | Dimmed 50% + no cursor | Clear non-interactive signal |
| Toasts | Auto-dismiss info, persist errors | Severity-appropriate behavior |
| Panel resize | Drag + double-click presets | Flexible + quick access |
| Panel persistence | Yes | Remembers user preferences |
| Region toggle | Instant (no animation) | Snappy feel |
| Panel tab order | Drag to reorder | User customizable |
