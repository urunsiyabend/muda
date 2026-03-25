---
phase: 08
plan: 06
name: panel-manager
subsystem: views
tags: [rust, panel-manager, bottom-panel, tabs, resize, presets]

dependency-graph:
  requires:
    - "08-02 (Tab element pattern for panel tabs)"
    - "07-01 (TabBarView/StatusBarView as layout reference)"
    - "06-design-system (ColorToken, Theme)"
  provides:
    - "PanelManagerView — bottom panel system with tabs and resize"
    - "PanelKind — enum for Output/Problems/Terminal/Debug"
    - "PanelState — panel layout state with height/collapsed/tab order"
    - "PanelPreset — collapsed/half/full preset cycling"
  affects:
    - "08-08 (AppLayout integrates PanelManager at bottom)"

tech-stack:
  added: []
  patterns:
    - "Preset cycling: Collapsed -> Half(200) -> Full(400) -> Collapsed"
    - "Height clamping: min=100, max=500 pixels"
    - "Resize handle: 4px hitbox, 1px visual line, hover highlight in Accent"
    - "Tab bar: inline Div tabs with active Accent bottom border"

key-files:
  created:
    - "ora/src/views/panel_manager.rs — PanelManagerView, PanelKind, PanelState, PanelPreset"
  modified:
    - "ora/src/views/mod.rs — registered panel_manager module"
    - "ora/src/lib.rs — added PanelManagerView re-exports"
---

## What was built

**PanelManagerView** — Bottom panel system with tabs for Output, Problems, Terminal, Debug.

### Components

1. **PanelKind** — Enum for panel types: Output, Problems, Terminal, Debug. Each has a `label()` method.

2. **PanelState** — Layout state: height (clamped 100-500px), is_collapsed, preferred_height, active_tab, tab_order.

3. **PanelPreset** — Collapsed/Half(200px)/Full(400px) for double-click cycling.

4. **PanelManagerView** — View rendering:
   - Resize handle at top (4px Div with 1px border line, hover Accent highlight)
   - Tab bar row with active/inactive styling (Accent bottom border for active)
   - Content area with placeholder text per panel type
   - Methods: set_height, toggle_collapsed, cycle_preset, set_active_tab, reorder_tab

### Commits

| Hash | Message |
|------|---------|
| dae1111 | feat(08-06): add PanelManager types and state management |
| f384573 | feat(08-06): add PanelManagerView rendering |

### Test Results

- test_panel_manager_creation — default state verification
- test_panel_preset_cycling — collapsed/half/full/collapsed cycle
- test_panel_height_clamping — min/max enforcement
- test_panel_tab_reorder — swap tab positions
- All 106 lib tests pass

### Decisions

- DEFAULT_PANEL_HEIGHT=200px, MIN=100px, MAX=500px
- Drag-to-resize renders visually (hover feedback) but actual drag tracking deferred to AppLayout integration
- Tab bar uses inline Div rendering (not Tab element) since panel tabs are simpler than file tabs
- Content area shows placeholder text per panel type
