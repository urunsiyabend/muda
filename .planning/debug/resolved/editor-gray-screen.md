---
status: resolved
trigger: "wgpu_client editor shows mostly gray screen, phase8_demo renders fine"
created: 2026-03-25T00:00:00Z
updated: 2026-03-25T00:10:00Z
---

## Current Focus

hypothesis: CONFIRMED - multiple wiring issues in EditorRootView
test: N/A - fixes applied
expecting: N/A
next_action: commit and archive

## Symptoms

expected: Full IDE layout with gutter, tab bar, text area, status bar (like phase8_demo)
actual: Mostly gray screen, 14+ hitboxes exist but limited visible content
errors: None (cargo check passes, 150 tests pass)
reproduction: cargo run -p wgpu_client
started: After EditorRootView was created

## Eliminated

(none - first hypothesis was correct)

## Evidence

- timestamp: 2026-03-25T00:01
  checked: editor_root.rs build_layout vs phase8_demo render
  found: AppLayout.sidebar_visible defaults to true but model.sidebar.visible is false
  implication: Layout included invisible sidebar div, layout state inconsistent

- timestamp: 2026-03-25T00:02
  checked: core_editor App::new() -> build_render_model(40)
  found: Returns valid data (1 line, gutter visible, caret visible, 1 tab)
  implication: Core data correct; issue is purely in EditorRootView wiring

- timestamp: 2026-03-25T00:03
  checked: DialogView::new called every frame with cx.focus_handle()
  found: FocusId leak - new handles allocated per frame despite persistent handles existing
  implication: Performance issue, not visual but should be fixed

- timestamp: 2026-03-25T00:04
  checked: FileTreePresentation always default (empty)
  found: file_tree_from_sidebar exists in adapter but was never called
  implication: When directory is opened, sidebar would show "No folder open" despite entries

## Resolution

root_cause: >
  EditorRootView had three wiring issues:
  1. sidebar_visible not synced from model data (AppLayout default=true, model=false)
  2. DialogView created with new focus handles every frame (FocusId leak)
  3. FileTreePresentation always empty (file_tree_from_sidebar never called)
fix: >
  1. Sync layout.sidebar_visible from model.sidebar.visible
  2. Add DialogView::with_focus_handles() and use persistent handles
  3. Build FileTreePresentation from sidebar entries in build_file_tree()
  4. Remove redundant file_tree_from_sidebar from adapter.rs
verification: cargo check --workspace clean, 150 ora + 104 core_editor tests pass
files_changed:
  - ora/src/views/editor_root.rs
  - ora/src/views/dialog.rs
  - wgpu_client/src/adapter.rs
  - core_editor/src/app.rs (added regression test)
