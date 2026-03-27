# Phase 13 Plan 04: Selection Polish + Verification Summary

**One-liner:** Focus-aware selection dimming, gutter click line select, I-beam cursor, tab close bug fix, human verification of all 7 SEL requirements.

## What Was Done

### Task 1: Selection dimming on focus loss and gutter click line select
- Added `SharedFocusState` (Rc<Cell<bool>>) shared between event loop and EditorRootView
- `WindowEvent::Focused` handler updates shared state and requests redraw
- TextAreaView uses `ColorToken::SelectionInactive` when `!editor_focused`
- Added `GutterClickAt { line }` command variant to both ora and core_editor
- Dispatcher `handle_gutter_click` selects entire line (anchor=line start, head=next line start)
- Event loop `pixel_to_gutter_line` detects clicks in gutter region
- I-beam cursor icon shown when hovering over text area

### Task 2: Human Verification Checkpoint
- All 22 verification items tested and approved by user
- SEL-01 through SEL-07 confirmed working

## Deviations from Plan

| Deviation | Type | Rationale |
|-----------|------|-----------|
| Added I-beam cursor icon for text area | Auto-add (user request) | Standard editor UX — pointer cursor is not suitable for text selection |
| Fixed tab close targeting wrong tab | Auto-fix (bug) | on_close dispatched CloseTab (active tab) instead of switching to target tab first |
| Reverted click-below-last-line EOF snap | User feedback | User preferred original column-snap behavior on last line |

## Commits

| Hash | Message |
|------|---------|
| 99dfecb | feat(13-04): selection focus dimming and gutter click line select |
| f8bbc96 | feat(13-04): text cursor (I-beam) icon over editor text area |
| 37b774e | fix(13-04): click below last line snaps to EOF, close correct tab |
| cdf5300 | revert(13-04): restore click-below-last-line to column snap behavior |

## Key Files

### Modified
- `ora/src/app.rs` — SharedFocusState type, wiring to EditorRootView and OraApp
- `ora/src/platform/event_loop.rs` — editor_has_focus tracking, Focused handler, gutter click, I-beam cursor
- `ora/src/views/editor_root.rs` — focus_state field, model.editor_focused set from shared state
- `ora/src/views/text_area.rs` — editor_focused field, SelectionInactive color when unfocused
- `ora/src/editor_adapter/types.rs` — GutterClickAt variant, editor_focused in RenderModel
- `core_editor/src/commands/editor_command.rs` — GutterClickAt variant
- `core_editor/src/commands/dispatcher.rs` — handle_gutter_click
- `wgpu_client/src/adapter.rs` — GutterClickAt routing, editor_focused default
- `ora/src/views/tab_bar.rs` — SwitchTab before CloseTab in on_close handler

## Test Results

- core_editor: 142 passed (single-threaded; clipboard race in multi-threaded is pre-existing)
- Full workspace: builds clean
