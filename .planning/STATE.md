# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** A functional, performant code editor built on ora's GPU-accelerated UI framework.
**Current focus:** v2.0 Functional Editor — Phase 14 complete, next: Phase 15 (Context Menu / File Operations).

## Current Position

Phase: 14 — File Browser (Complete)
Plan: 04 of 4 (complete)
Status: Phase complete — Plan 14-04 done
Last activity: 2026-03-27 — Completed 14-04-PLAN.md (External file changes + verification)

Progress: [████████████████████████░░░░░░░░░░░░] v2.0 Phase 14 complete (23/~28 plans)

## Performance Metrics

**v1.0 Velocity (archived):**
- Total plans completed: 40 (Phase 1: 3, Phase 2: 6, Phase 3: 4, Phase 4: 5, Phase 5: 5, Phase 6: 4, Phase 7: 5, Phase 8: 8, Phase 8.1: 3, Phase 9: 9)
- Average duration: ~7.5m per plan
- Total execution time: ~5 hours 34 minutes

*Updated after each plan completion*

### Phase 12 Gap Closure Decisions (Plan 04)

- Field-level unsafe cast (individual fields, not whole struct) to avoid `invalid_reference_casting` deny lint in Rust — cast `*.pending_status_message` and `*.status_message_expiry` separately
- status_message_expiry set at every pending_status_message assignment site (5 total) to guarantee 3s lifetime for all messages
- about_to_wait uses `earliest_wake = min(blink_instant, message_expiry)` so event loop wakes exactly once at the sooner deadline

### Phase 14 Plan 01 Decisions

- ignored_patterns exact name match (p == name) — only .git hidden, .gitkeep/.gitignore visible
- auto_expand_first_level called from both new_with_patterns() and set_base_directory()
- AppState private struct; only load_workspace_path() is pub for main.rs
- open_directory() in adapter injects ignored_patterns via Sidebar::new_with_patterns() — avoids changing core_editor::App API

### Phase 14 Plan 02 Decisions

- has_no_workspace() sentinel checks directory_name == "Files" (core_editor fallback) — avoids adding new boolean field
- Sidebar header shows directory_name.to_uppercase(); falls back to "EXPLORER" when no workspace
- Empty state Open Folder button uses existing Button element (secondary variant) + sidebar dispatch callback
- Ctrl+Shift+O guard placed before Ctrl+O in character match (same pattern as Ctrl+Shift+S before Ctrl+S)
- handle_folder_opened calls sidebar.show() to ensure sidebar becomes visible after picking folder

### Phase 14 Plan 03 Decisions

- notify::RecursiveMode accessed via notify_debouncer_full::notify::RecursiveMode — no direct notify dep needed
- debouncer.watch() directly (not debouncer.watcher().watch()) — watcher() is deprecated in 0.7
- poll_watcher_events placed BEFORE dirty-entity check in about_to_wait so watcher events get full layout pass
- Drop-then-recreate pattern for watcher restart: self.watcher=None before new Debouncer

### Phase 14 Plan 04 Decisions

- deleted_paths HashSet on adapter (not core_editor domain) — avoids propagating filesystem concerns into domain, O(1) lookup per tab
- pending_reload_prompts Vec dequeues one prompt at a time — prevents dialog stacking on simultaneous external changes
- Muted color for deleted tab indicator — no text layout changes needed, communicates deletion clearly
- needs_layout=true on ALL scroll events — proven necessary by LayoutId count mismatch debug logs when visible_lines count varies per frame
- Scissor rect y+h clamped to surface_height — GPU validation rejects out-of-bounds scissor rects
- Scroll hitbox: subtract scroll_offset from y-origin + intersect with viewport rect — prevents off-screen hitboxes stealing events

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
v1.0 decisions carried forward — see MILESTONES.md for full history.

### Pending Todos

None.

### Roadmap Evolution

- 2026-03-26: v2.0 roadmap created with Phases 10-16 (7 phases, 40 requirements)
  - Source material stated "34 requirements" but actual count is 40: FIX(5) + TAB(5) + FILE(6) + SIDE(6) + SEL(7) + FIND(8) + PERF(3)
- 2026-03-27: Phase 13.1 inserted after Phase 13: Rendering Performance Optimization (URGENT)
  - Frame time 10-26ms with 10 tabs open (release), layout is 80% of frame cost
  - Per-frame element tree rebuild + no layout caching + no viewport virtualization
  - Must fix before Phase 14 (File Browser) which will add more sidebar elements
  - Research: Zed GPUI source code, blog posts on retained rendering and element caching
  - RESULT: avg 1.8ms frame time, max 3.5ms, 98% glyph cache, 59% layout cache

### Blockers/Concerns

- Async file I/O return path: RESOLVED (12-02) — Full flow: dispatch_command queues PendingFileOp → poll_pending_file_ops() → LocalExecutor future → background thread reads file → adapter callback delivers result; rfd AsyncFileDialog + futures-lite yield_now pattern
- FileOpDataSource sub-trait: RESOLVED (12-02) — file-op callbacks isolated in FileOpDataSource sub-trait; EditorDataSource super-trait updated; PendingFileOp relocated to ora
- Save / Save As dialog flow: RESOLVED (12-03) — PendingFileOp::Save routes through event loop for has-path decision; spawn_save_as_dialog uses rfd::AsyncFileDialog::save_file; handle_file_saved updates path, clears dirty, registers buffer registry; last_dir persisted to state.json
- EditorDataSource trait pressure: RESOLVED (10-02) — split into BufferDataSource + CommandDispatcher + WindowDataSource with blanket super-trait
- Selection rendering: RESOLVED (10-03) — selection_ranges in LinePresentation, 5-layer Stack, span filter removed
- Idle GPU fixed (10-01): unconditional request_redraw at event_loop.rs removed; ControlFlow state machine now drives frame cadence

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug
- Window close (X button) doesn't check for dirty documents — exits without save dialog
- Dialog butonları (Save/Don't Save/Cancel) tıklanamıyor — hitbox/event wiring eksik
- Dispatch system fires handlers in both capture+bubble phases — all handlers must check `ctx.phase() == Bubble`
- Scroll perf: full layout triggered every scroll frame because visible_lines count varies per frame (viewport virtualization). Documented for Phase 13.2 (Rendering Performance — View-level Dirty Checking).

## Phase 12 Summary

Plan 01: rfd 0.15 + dirs 5 deps, untitled naming, PendingFileOp queue, dialog_open guard, Ctrl+N, Ctrl+S queues SaveAs
Plan 02: Ctrl+O native dialog, background file reading, UTF-8 validation, BOM strip, Buffer Registry dedup, FileOpDataSource sub-trait, poll_pending_file_ops
Plan 03: Ctrl+S silent save / Save As dialog (rfd save_file), handle_file_saved callback, last-dir persisted to state.json, open+save dialogs pre-populate directory

Plan 04 (gap closure): Dialog input suppression for CursorMoved/MouseInput/MouseWheel/KeyboardInput; 3-second timed status messages with Instant expiry; about_to_wait earliest-wake scheduling

## Phase 13 Summary

Plan 01: ClickAt/DragTo command pipeline, pixel_to_doc conversion with mid-character snap, word/line selection on double/triple-click, drag-to-select, 7 new tests
Plan 02: Mouse drag with 3px threshold, word/line snap modes in DragTo, scroll-while-drag at viewport edges, TextBuffer::word_boundary_at, 10 new tests
Plan 03: Full-line copy/cut with no selection, paste-above for line copies, auto-indent multi-line paste, clipboard_is_line_copy flag, 10 new tests
Plan 04: Selection focus dimming (SelectionInactive), gutter click line select, I-beam cursor, tab close bug fix, human verification approved

## Phase 13.1 Summary

Plan 01: GlyphCacheKey + LRU glyph buffer cache (2048-cap, peek+clone), TextElement wired to measure_text_cached + paint_text_cached, --no-glyph-cache flag, glyph hit % in --show-fps (98% steady-state)
Plan 02: needs_layout dirty flag, cached layout_outputs reuse (skip compute_flexbox on paint-only frames), request_layout always runs for LayoutId assignment, --no-layout-cache flag, layout hit % in --show-fps (59-63%)
Plan 03: FileTreeView viewport virtualization — virtual_slice() with 20-row buffer zones, spacer divs, TREE_ITEM_HEIGHT pub const, SharedScrollState → sidebar → FileTreeView; 5 new tests
Plan 04: FrameDegradation guard (3+ slow frames suppress animations), --no-cache umbrella flag, [DEGRADED] indicator, human verification passed (avg 1.8ms, max 3.5ms, 0 degraded)

## Phase 14 Summary (in progress)

Plan 01: ignored_patterns exact-match filter (.git hidden, dotfiles visible), state.json extended with workspace_path + ignored_patterns, startup workspace restore, auto_expand_first_level, mark_tree_dirty() public
Plan 02: Open Folder dialog via Ctrl+Shift+O + rfd pick_folder, sidebar header shows workspace name, empty state with Open Folder button, handle_folder_opened() + sidebar.show()
Plan 03: notify-debouncer-full 0.7 watcher in CoreEditorAdapter, poll_watcher_events/start_watcher/stop_watcher on FileOpDataSource trait, 300ms debounce, polling in about_to_wait, live sidebar refresh
Plan 04: External file deletion (deleted_paths HashSet → TabPresentation.is_deleted → muted tab indicator), clean-buffer auto-reload, dirty-buffer ExternalModificationPrompt dialog, workspace switch closes tabs; 5 rendering bug fixes (scissor clamping, scroll hitbox offset, hitbox viewport clip, needs_layout on scroll, layout cache growth guard), Phase 14 all 6 SC verified

## Session Continuity

Last session: 2026-03-27
Stopped at: Completed 14-04-PLAN.md (Phase 14 complete)
Resume file: None
Next: Phase 15 — Context Menu / File Operations (or Phase 13.2 Rendering Performance if scroll perf is prioritized)

---
*State initialized: 2026-01-28*
*Last updated: 2026-03-27 after Phase 13.1 completion*
