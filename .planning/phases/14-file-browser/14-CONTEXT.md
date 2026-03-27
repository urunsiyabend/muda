# Phase 14: File Browser - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can navigate the project directory tree in the sidebar and open files by clicking. The tree reflects real filesystem state via a file watcher. Includes Open Folder to set workspace root. Does NOT include file creation/deletion/rename UI, search, or multi-root workspaces.

</domain>

<decisions>
## Implementation Decisions

### Tree presentation
- Sorting: folders first, then files — both groups alphabetical within their group
- File type icons: distinct icons per file type (Rust, TOML, Markdown, etc.), not just generic folder/file
- Expand/collapse indicator: right arrow when collapsed, down arrow when expanded
- No active file highlighting — tree is purely for navigation, no indication of which file is currently open
- First level auto-expanded on workspace open — root children visible immediately

### Ignore & filter rules
- Only `.git/` hidden by default — everything else visible including target/, node_modules/
- Configurable ignore list stored in `state.json` as an `ignored_patterns` array
- Dotfiles (`.env`, `.rustfmt.toml`, etc.) are visible by default
- No in-app UI for managing ignore list in this phase — manual state.json editing only (settings UI deferred to future)

### File watcher behavior
- Auto-refresh via filesystem watcher (notify crate) — tree updates in real time on external changes
- Debounce rapid changes (e.g., cargo build) — batch filesystem events, refresh tree at most every ~200-500ms
- If an open file is deleted externally: keep tab open, mark as deleted (visual indicator, content preserved)
- If an open file is modified externally: auto-reload if buffer has no unsaved changes; prompt if dirty (modern IDE behavior — VS Code/IntelliJ/Zed standard)

### Workspace root experience
- Startup: restore last opened workspace if available, otherwise show empty state with "No folder open" + Open Folder button
- Open Folder available via: dedicated keybinding + command palette + sidebar empty state button
- Opening a new folder closes all tabs, but prompts to save any dirty buffers first
- Sidebar header shows workspace root folder name (e.g., "muda") as a title at the top

### Claude's Discretion
- Exact keybinding for Open Folder (Ctrl+K Ctrl+O or similar)
- File type icon set and visual design
- Arrow indicator exact glyphs/rendering
- Debounce timing within the ~200-500ms range
- How "file deleted" indicator looks on the tab
- Reload prompt dialog design

</decisions>

<specifics>
## Specific Ideas

- External file modification handling should follow modern IDE conventions (auto-reload clean buffers, prompt for dirty ones)
- Settings UI for ignore patterns is planned for a future phase — for now, manual state.json editing is acceptable

</specifics>

<deferred>
## Deferred Ideas

- In-app settings UI for configuring ignore patterns — future settings/preferences phase
- Multi-root workspaces — potential future capability
- File creation/deletion/rename from sidebar context menu — separate phase

</deferred>

---

*Phase: 14-file-browser*
*Context gathered: 2026-03-27*
