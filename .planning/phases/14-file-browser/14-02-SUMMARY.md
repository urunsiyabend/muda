---
phase: 14-file-browser
plan: 02
subsystem: ui
tags: [rust, ora, sidebar, file-browser, rfd, dialog, keybinding]

# Dependency graph
requires:
  - phase: 14-01
    provides: "Sidebar::set_base_directory() + auto_expand_first_level, workspace_path state persistence, ignored_patterns"
  - phase: 12-file-operations
    provides: "PendingFileOp queue pattern, rfd::AsyncFileDialog, FileOpDataSource trait, spawn dialog pattern"
provides:
  - "EditorCommand::OpenFolder + PendingFileOp::OpenFolder variant"
  - "FileOpDataSource::handle_folder_opened() callback"
  - "spawn_open_folder_dialog() in event loop using rfd::AsyncFileDialog::pick_folder"
  - "Ctrl+Shift+O keybinding -> OpenFolder"
  - "Open Folder in command palette (Ctrl+Shift+P)"
  - "Sidebar header shows workspace folder name instead of static EXPLORER"
  - "Sidebar empty state: No folder open + Open Folder button dispatching OpenFolder"
affects:
  - "14-03 (filesystem watcher may need to know when workspace changes)"
  - "14-04 (UAT will verify open folder dialog flow)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PendingFileOp::OpenFolder queued by dispatch_command, consumed by poll_pending_file_ops, spawns async pick_folder dialog"
    - "has_no_workspace() sentinel: directory_name == 'Files' (core_editor default) means no base dir set"
    - "Sidebar dispatch callback (Rc<dyn Fn(EditorCommand)>) reused for empty state button"

key-files:
  created: []
  modified:
    - "ora/src/editor_adapter/types.rs - PendingFileOp::OpenFolder + EditorCommand::OpenFolder"
    - "ora/src/editor_adapter/mod.rs - FileOpDataSource::handle_folder_opened() trait method"
    - "ora/src/events/editor_input.rs - Ctrl+Shift+O -> OpenFolder keybinding"
    - "ora/src/platform/event_loop.rs - PendingFileOp::OpenFolder arm + spawn_open_folder_dialog()"
    - "ora/src/views/editor_root.rs - Open Folder CommandItem in palette registry"
    - "ora/src/views/sidebar.rs - dynamic header title + empty state with Open Folder button"
    - "wgpu_client/src/adapter.rs - OpenFolder in dispatch_command + handle_folder_opened impl"

key-decisions:
  - "Use has_no_workspace() sentinel (directory_name == 'Files') rather than separate flag — avoids new state"
  - "Header label is uppercased directory_name to match VS Code EXPLORER style"
  - "Open Folder button uses existing Button element (secondary variant) + sidebar dispatch callback"
  - "Ctrl+Shift+O bound with shift guard before plain Ctrl+O in the character match arm"
  - "Command palette entry uses 'file.open-folder' id with Ctrl+Shift+O shortcut hint"

patterns-established:
  - "OpenFolder follows exact same async dialog pattern as Open/SaveAs: queue PendingFileOp -> poll -> spawn"
  - "Sidebar empty states differentiated: has_no_workspace -> Open Folder CTA; folder open but empty dir -> neutral placeholder"

# Metrics
duration: 6min
completed: 2026-03-27
---

# Phase 14 Plan 02: Open Folder Dialog + Sidebar Chrome Summary

**OpenFolder dialog via Ctrl+Shift+O + rfd pick_folder, sidebar header shows workspace name, empty state guides user with Open Folder button**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-03-27T18:55:38Z
- **Completed:** 2026-03-27T19:01:18Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Open Folder command fully wired: Ctrl+Shift+O keybinding -> EditorCommand::OpenFolder -> PendingFileOp::OpenFolder -> spawn_open_folder_dialog() -> rfd::AsyncFileDialog::pick_folder -> handle_folder_opened() -> set_base_directory + persist workspace_path
- Sidebar header dynamically shows workspace folder name (uppercased) instead of static "EXPLORER"; falls back to EXPLORER when no folder is open
- Empty sidebar state shows "No folder open" with subtitle and functional "Open Folder" secondary button that dispatches OpenFolder through the sidebar's existing dispatch callback
- "Open Folder" added to command palette registry with Ctrl+Shift+O shortcut hint

## Task Commits

Each task was committed atomically:

1. **Task 1: Add OpenFolder command + PendingFileOp variant + dialog** - `874c4c0` (feat)
2. **Task 2: Sidebar header with workspace name + empty state Open Folder button** - `d7aa767` (feat)

## Files Created/Modified

- `ora/src/editor_adapter/types.rs` - Added PendingFileOp::OpenFolder and EditorCommand::OpenFolder variants
- `ora/src/editor_adapter/mod.rs` - Added handle_folder_opened() to FileOpDataSource trait
- `ora/src/events/editor_input.rs` - Ctrl+Shift+O -> OpenFolder keybinding (before Ctrl+O guard)
- `ora/src/platform/event_loop.rs` - OpenFolder match arm in poll_pending_file_ops + spawn_open_folder_dialog()
- `ora/src/views/editor_root.rs` - "Open Folder" CommandItem added to palette registry
- `ora/src/views/sidebar.rs` - Dynamic header title + has_no_workspace() + empty state with button
- `wgpu_client/src/adapter.rs` - OpenFolder in dispatch_command + handle_folder_opened() implementation

## Decisions Made

- `has_no_workspace()` sentinel checks `directory_name == "Files"` (core_editor's fallback when no base directory) — avoids adding a new boolean field
- Header title is `directory_name.to_uppercase()` to match VS Code EXPLORER style; "EXPLORER" when no workspace
- Open Folder button in empty state uses the existing `Button` element (secondary variant) and the sidebar's `dispatch` callback — no new mechanism needed
- Ctrl+Shift+O guard placed before Ctrl+O in the character match arm (same pattern as Ctrl+Shift+S before Ctrl+S)
- `handle_folder_opened` calls `std::fs::canonicalize` for path normalization and calls `sidebar.show()` to ensure the sidebar becomes visible after picking a folder

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `Div::on_mouse_down()` does not exist in ora's Div element; the plan suggested it. Switched to `crate::elements::button()` (secondary variant) with `on_click()` — this is the correct element-level pattern used throughout the codebase for interactive buttons. Fixed immediately during Task 2, no scope impact.

## Next Phase Readiness

- Open Folder flow fully functional: keybinding, command palette, dialog, sidebar refresh, state persistence
- Phase 14-03 (filesystem watcher) can build on the workspace_path now set by handle_folder_opened
- Phase 14-04 (UAT) can verify the end-to-end open folder experience

---
*Phase: 14-file-browser*
*Completed: 2026-03-27*
