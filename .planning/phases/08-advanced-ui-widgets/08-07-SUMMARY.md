---
phase: 08
plan: 07
name: overlay-widgets
subsystem: elements
tags: [rust, context-menu, toast, overlay, notifications]

dependency-graph:
  requires:
    - "08-02 (ListItem styling pattern for menu items)"
    - "06-design-system (ColorToken including Success, Error, Warning)"
    - "07-03 (Stack z-layering for overlay rendering)"
  provides:
    - "ContextMenu — popup menu with items, separators, keyboard nav"
    - "ContextMenuItem — action items with shortcuts and separators"
    - "Toast — notification container with severity-based auto-dismiss"
    - "ToastNotification — individual notification with timer"
    - "ToastSeverity — Info/Success/Warning/Error severity levels"
  affects:
    - "08-08 (AppLayout integrates overlays via Stack)"

tech-stack:
  added: []
  patterns:
    - "dismiss_at: Option<Instant> for timer-based auto-dismiss (no cx.spawn)"
    - "tick() pattern: caller invokes from event loop to remove expired toasts"
    - "Viewport clamping: clamp_to_viewport() adjusts menu position near edges"
    - "Severity color mapping: ToastSeverity -> ColorToken"

key-files:
  created:
    - "ora/src/elements/context_menu.rs — ContextMenu, ContextMenuItem"
    - "ora/src/elements/toast.rs — Toast, ToastNotification, ToastSeverity"
  modified:
    - "ora/src/elements/mod.rs — registered context_menu and toast modules"
    - "ora/src/lib.rs — added ContextMenu, Toast re-exports"
---

## What was built

**ContextMenu** and **Toast** overlay widgets for popups and notifications.

### ContextMenu

Popup menu rendering at a given position with:
- Action items: label, optional shortcut, disabled state
- Separators: visual dividers between item groups
- Keyboard navigation: move_up/move_down skip separators, wrap around
- Viewport clamping: clamp_to_viewport() adjusts position near edges
- Visual: BgElevated background, Border outline, drop shadow, 200px width

### Toast

Notification container with severity-based auto-dismiss:
- **Info/Success**: Auto-dismiss after 4 seconds via dismiss_at timestamp
- **Warning/Error**: Persist until user dismisses (dismiss button shown)
- Severity accent strip: colored left bar (Accent/Success/Warning/Error)
- Stack rendering: bottom-right positioning, multiple toasts stack vertically
- tick() method: remove expired notifications (called from event loop)

### Commits

| Hash | Message |
|------|---------|
| 943b596 | feat(08-07): add ContextMenu and Toast overlay widgets |

### Test Results

- test_context_menu_navigation — up/down skips separators
- test_context_menu_viewport_clamping — position adjusted near edges
- test_toast_auto_dismiss — info toast has dismiss_at set
- test_toast_persist — error toast has no dismiss_at
- test_toast_severity_colors — each severity maps to correct token
- All 106 lib tests pass

### Decisions

- ContextMenuItem does not derive Clone (Box<dyn Fn()> prevents it) — items created fresh each render
- Toast uses Instant-based timers instead of async spawn — simpler, no executor dependency
- AUTO_DISMISS_DURATION=4 seconds for info/success
- ContextMenu.render() returns AnyElement for Stack compatibility
- Toast container positioned by parent (AppLayout uses Stack), not self-positioning
