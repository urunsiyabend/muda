# Phase 11: Buffer Registry + Multi-Tab - Research

**Researched:** 2026-03-26
**Domain:** Rust editor architecture — Workspace, Buffer Registry, tab management
**Confidence:** HIGH (all findings from direct codebase inspection)

---

## Summary

Phase 11 operates on an already-mature architecture. Workspace, Document, EditorView,
TabBarPresentation, and DialogPresentation types exist and are wired. The UI tab bar
renders from TabPresentation data. The adapter stubs out SwitchTab and CloseTab.

The work is filling three specific structural gaps:

1. Workspace has no path-to-DocumentId index. Opening the same file creates a duplicate Document.
2. Views live in a `HashMap<ViewId, EditorView>` — iteration order is random, breaking visual tab order.
3. No MRU stack exists — close focus follows random HashMap iteration.

Every other requirement (per-buffer scroll/cursor state, dirty tracking, dialog, presentation
types) is already implemented and just needs to be plumbed through correctly.

**Primary recommendation:** Add three fields to Workspace, fix open_document, fix tab iteration,
wire existing stubs. Do not rebuild anything that already works.

---

## Standard Stack

This phase adds no external libraries. It is internal Rust architecture work.

### What Already Exists (Do Not Rebuild)

| Component | Location | Status |
|-----------|----------|--------|
| EditorView (cursor, scroll, selection per-view) | `core_editor/src/view/editor_view.rs` | Complete |
| TabBarPresentation / TabPresentation types | `core_editor/src/view_model/mod.rs` | Complete |
| TabBarView renderer | `ora/src/views/tab_bar.rs` | Complete |
| DialogPresentation::UnsavedChangesConfirmation | `core_editor/src/view_model/mod.rs` | Complete |
| SwitchTab(u64) / CloseTab in EditorCommand (ora) | `ora/src/editor_adapter/types.rs` | Stubbed |
| ProtectedResult / DiscardAcknowledgment | `core_editor/src/domain/protection.rs` | Complete |
| Empty document fallback in build_render_model | `core_editor/src/app.rs` line 700 | Exists |

### What Needs to Be Added

| Component | Location | Work |
|-----------|----------|------|
| Buffer Registry: `HashMap<PathBuf, DocumentId>` | `Workspace` struct | New field |
| Tab ordering: `Vec<ViewId>` | `Workspace` struct | New field |
| MRU stack: `Vec<ViewId>` | `Workspace` struct or `App` | New field |
| SwitchTab handler in App | `core_editor/src/app.rs` | New method |
| CloseTab handler in App | `core_editor/src/app.rs` | New method |
| Wire stubs in adapter | `wgpu_client/src/adapter.rs` | Replace stub with real call |
| Ctrl+Tab / Ctrl+W keybindings | `wgpu_client/src/main.rs` | New key handling |
| Dirty indicator: dot before filename | `ora/src/views/tab_bar.rs` | Visual change |

---

## Architecture Patterns

### Pattern 1: Buffer Registry (HashMap<PathBuf, DocumentId>)

**What:** A path index on Workspace that prevents duplicate Document creation.

**Field to add to Workspace struct:**

```rust
// In core_editor/src/domain/workspace.rs
pub struct Workspace {
    documents: HashMap<DocumentId, Document>,
    views: HashMap<ViewId, EditorView>,
    histories: HashMap<DocumentId, CommandHistory>,
    active_view_id: Option<ViewId>,
    event_bus: EventBus,

    // NEW — Buffer Registry
    path_to_doc: HashMap<PathBuf, DocumentId>,

    // NEW — visual tab ordering (insertion order maintained)
    tab_order: Vec<ViewId>,

    // NEW — MRU stack (most recently activated at front)
    mru_stack: Vec<ViewId>,
}
```

**Canonicalization requirement:** Always canonicalize paths before inserting into or
looking up from `path_to_doc`. Use `std::fs::canonicalize()` for real files. Handle
the canonicalize-failure case gracefully (path doesn't exist yet — use as-is).

**Modified open_document:**

```rust
pub fn open_document(&mut self, path: &Path) -> std::io::Result<(DocumentId, bool)> {
    // Returns (doc_id, was_existing) to let callers decide behavior.
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    if let Some(&existing_id) = self.path_to_doc.get(&canonical) {
        return Ok((existing_id, true));  // Already open — reuse
    }

    let document = Document::open(path)?;
    let doc_id = document.id();
    self.event_bus.publish(document.event_opened());
    self.documents.insert(doc_id, document);
    self.histories.insert(doc_id, CommandHistory::new());
    self.path_to_doc.insert(canonical, doc_id);  // Register in registry
    Ok((doc_id, false))
}
```

**When file already exists:** return `(existing_id, true)`. App-level code then calls
`find_or_create_view_for_doc` and switches to that view instead of creating a new one.

### Pattern 2: Tab Ordering (Vec<ViewId>)

**Problem:** `HashMap<ViewId, EditorView>` has unpredictable iteration order. Tab bar
must render in a stable, insertion-ordered sequence. Ctrl+Tab must cycle left-to-right
in visual order.

**What:** Maintain a parallel `Vec<ViewId>` that represents the visual tab strip order.
All view creation/close operations must keep this vec in sync.

**Key invariant:** Every ViewId in views HashMap must be in tab_order exactly once.

**Modified create_view:**

```rust
pub fn create_view(&mut self, doc_id: DocumentId) -> ViewId {
    let view = EditorView::new(doc_id);
    let view_id = view.id();

    self.event_bus.publish(view.event_created());
    self.views.insert(view_id, view);

    // Insert after current active tab, not at end
    if let Some(active_id) = self.active_view_id {
        if let Some(pos) = self.tab_order.iter().position(|&id| id == active_id) {
            self.tab_order.insert(pos + 1, view_id);
        } else {
            self.tab_order.push(view_id);
        }
    } else {
        self.tab_order.push(view_id);
    }

    // Push to MRU stack front
    self.mru_stack.retain(|&id| id != view_id);
    self.mru_stack.insert(0, view_id);

    self.active_view_id = Some(view_id);
    view_id
}
```

**Modified close_view:** Remove from tab_order AND mru_stack on close.

**Modified get_open_views_info (app.rs):** Must iterate `self.workspace.tab_order()` not
`self.workspace.views()` to produce tabs in correct visual order.

### Pattern 3: MRU Stack

**What:** A separate `Vec<ViewId>` tracking activation history. Front = most recent.

**When to push/promote:** On every `set_active_view()` call, move the view_id to the
front of mru_stack.

**When to use:** After closing active tab, select `mru_stack[0]` as next active (after
removing the closed view from the stack).

**Implementation in set_active_view:**

```rust
pub fn set_active_view(&mut self, view_id: ViewId) -> bool {
    if self.views.contains_key(&view_id) {
        self.active_view_id = Some(view_id);
        // Promote to front of MRU
        self.mru_stack.retain(|&id| id != view_id);
        self.mru_stack.insert(0, view_id);
        true
    } else {
        false
    }
}
```

### Pattern 4: Tab Commands Handled at App Level

**What:** SwitchTab and CloseTab do NOT go through the command dispatcher (they modify
Workspace structure, not document content). Like Save, they are App-level operations.

**In adapter.rs dispatch_command:** Replace stub messages with real App method calls.

```rust
EditorCommand::SwitchTab(view_id) => {
    self.app.switch_tab(view_id);
    return;
}
EditorCommand::CloseTab => {
    self.app.close_active_tab();
    return;
}
```

**App::switch_tab must NOT call can_switch_active().** Direct call to
`workspace.set_active_view(ViewId::from_raw(view_id))`. The dirty state of the
buffer being left is irrelevant for tab switching — the user can have dirty buffers
open.

**App::close_active_tab:** Call `close_document_protected` for the document. If dirty,
set `pending_action = PendingAction::CloseDocument(doc_id)` and trigger dialog. After
close, activate MRU top.

### Recommended Workspace Structure for Open Tabs Info

```rust
// In workspace.rs — expose ordered tab info
pub fn tab_order(&self) -> &[ViewId] {
    &self.tab_order
}

// In app.rs — replace get_open_views_info
fn get_open_views_info(&self) -> Vec<(u64, String, bool, bool)> {
    let active_view_id = self.workspace.active_view_id();
    self.workspace.tab_order()
        .iter()
        .filter_map(|view_id| {
            let view = self.workspace.view(*view_id)?;
            let doc = self.workspace.document(view.document_id())?;
            let is_active = Some(*view_id) == active_view_id;
            // Title without dirty prefix — dirty shown via dot indicator separately
            let title = doc.file_path()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[New File]")
                .to_string();
            Some((view_id.as_u64(), title, is_active, doc.is_dirty()))
        })
        .collect()
}
```

### Pattern 5: Empty State When Last Tab Closes

`build_render_model` in app.rs already has this branch:

```rust
_ => {
    // Even without a document, we might want to show the sidebar
    ViewModelBuilder::build_sidebar_only(&self.sidebar, self.focus, viewport_height)
}
```

When the last tab closes, `active_document()` and `active_view()` return None, and this
branch fires. Extend `build_sidebar_only` or the empty state as needed — do not create
a new code path.

### Anti-Patterns to Avoid

- **Calling can_switch_active() during tab switching:** This method returns an error if
  the active document is dirty. It was designed for Open File (Phase 7) where switching
  away from dirty doc without saving was the concern. For tab switching, dirty buffers
  are EXPECTED and should stay open. Never call can_switch_active() for SwitchTab.
- **Iterating views HashMap for tab order:** HashMap iteration order is random. Always
  use tab_order Vec.
- **Creating duplicate Documents for same path:** The entire point of Buffer Registry is
  to prevent this. All callers of open_document must go through the registry check.
- **Removing from tab_order but not mru_stack (or vice versa):** These two must stay in
  sync. Any close operation must purge the ViewId from both.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-buffer scroll state | Separate scroll HashMap | EditorView.viewport | Already stored per-view |
| Per-buffer cursor state | Cursor HashMap | EditorView.carets | Already stored per-view |
| Per-buffer selection state | Selection HashMap | EditorView.selections | Already stored per-view |
| Per-buffer undo history | Extra history store | workspace.histories HashMap | Already keyed by DocumentId |
| Dirty state tracking | Custom dirty flag | Document.is_dirty() | Already implemented |
| Save/Discard/Cancel dialog | Custom dialog widget | DialogPresentation::UnsavedChangesConfirmation | Type exists, dialog view renders it |

**Key insight:** Every piece of per-buffer state is already in EditorView or Document.
Tab switching is `set_active_view()` + redraw. No data needs to be saved or restored.

---

## Common Pitfalls

### Pitfall 1: can_switch_active() Called During Tab Switch

**What goes wrong:** Switch to different tab triggers "unsaved changes" dialog even
though the user just wants to look at another file.

**Why it happens:** `request_open_file` calls `can_switch_active()` before opening. If
this codepath is reused for tab switching, every switch on a dirty buffer shows a dialog.

**How to avoid:** SwitchTab dispatches to `App::switch_tab()` which calls
`workspace.set_active_view()` DIRECTLY. Never route through `request_open_file`.

**Warning signs:** Dialog appears when clicking a tab while editing.

### Pitfall 2: Tab Order Based on HashMap Iteration

**What goes wrong:** Tabs render in random order that changes between frames.

**Why it happens:** `workspace.views()` returns HashMap iterator with undefined order.
Current `get_open_views_info()` in app.rs uses this iterator.

**How to avoid:** Add `tab_order: Vec<ViewId>` to Workspace. Use only this vec for
tab presentation. Never use `workspace.views()` for building tab list.

**Warning signs:** Tabs jump around position when opening/closing other tabs.

### Pitfall 3: Duplicate Documents for Same File

**What goes wrong:** Opening same file twice creates two Document instances with same
content. Edits to one are not reflected in the other.

**Why it happens:** `open_document()` always creates a new Document without checking
if the path is already registered.

**How to avoid:** Buffer Registry check at top of open_document. Canonicalize path
before both insert and lookup.

**Warning signs:** Two tabs with same filename, edits in one not visible in other.

### Pitfall 4: Dirty Indicator Shows * Prefix Instead of Dot

**What goes wrong:** Tab title shows `*filename.rs` instead of `filename.rs` with a
dot indicator.

**Why it happens:** `Document::title()` currently formats dirty titles as `*{name}`.
TabBarView currently also adds `* ` prefix in `render_tab`. Both need changes.

**How to avoid:**
- In `get_open_views_info`: strip the dirty prefix from title (extract clean filename).
- In `TabBarView::render_tab`: render a dot element before filename, not a `* ` prefix.
- The `is_dirty` field on TabPresentation controls this — use it.

**Current code location:**
- `core_editor/src/domain/document.rs` line 274-287 (title() method adds `*`)
- `ora/src/views/tab_bar.rs` line 84-88 (renders `* {}` prefix)

### Pitfall 5: Close View Leaves Orphaned Entries

**What goes wrong:** MRU stack or tab_order contains a ViewId that no longer exists
in the views HashMap. Subsequent MRU lookups return a dead ID.

**Why it happens:** close_view removes from views HashMap but forgets to update
tab_order or mru_stack.

**How to avoid:** In `close_view`, after removing from views:
```rust
self.tab_order.retain(|&id| id != view_id);
self.mru_stack.retain(|&id| id != view_id);
```

### Pitfall 6: Path Canonicalization Failures

**What goes wrong:** Same file opened via `./foo.rs` and `/absolute/foo.rs` creates
two Document instances.

**Why it happens:** Storing path as-is without normalizing.

**How to avoid:** Use `std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())`
before every registry insert or lookup. The fallback handles non-existent paths gracefully.

---

## Code Examples

### Tab Close + MRU Focus Selection

```rust
// In core_editor/src/app.rs
pub fn close_active_tab(&mut self) {
    let active_view_id = match self.workspace.active_view_id() {
        Some(id) => id,
        None => return,
    };
    let active_doc_id = match self.workspace.view(active_view_id) {
        Some(v) => v.document_id(),
        None => return,
    };

    // Check if dirty — show dialog if so
    match self.workspace.close_document_protected(active_doc_id) {
        Ok(_) => {
            // Document closed; views also closed by close_document_force
            // Activate next via MRU
            self.activate_mru_after_close(active_view_id);
            self.needs_render = true;
        }
        Err(error) => {
            self.pending_action = Some(PendingAction::CloseDocument(active_doc_id));
            self.pending_error = Some(error.clone());
            self.needs_render = true;
        }
    }
}

fn activate_mru_after_close(&mut self, closed_view_id: ViewId) {
    // MRU stack has already removed closed_view_id in close_view
    if let Some(&next_id) = self.workspace.mru_stack().first() {
        self.workspace.set_active_view(next_id);
    }
    // If no views remain, active_view_id becomes None — empty state handled downstream
}
```

### Ctrl+Tab Navigation

```rust
// In core_editor/src/app.rs
pub fn switch_tab_next(&mut self) {
    let tab_order = self.workspace.tab_order();
    if tab_order.len() <= 1 { return; }
    let active = self.workspace.active_view_id();
    let current_pos = active
        .and_then(|id| tab_order.iter().position(|&v| v == id))
        .unwrap_or(0);
    let next_pos = (current_pos + 1) % tab_order.len();
    let next_id = tab_order[next_pos];
    self.workspace.set_active_view(next_id);
    self.needs_render = true;
}

pub fn switch_tab_prev(&mut self) {
    let tab_order = self.workspace.tab_order();
    if tab_order.len() <= 1 { return; }
    let active = self.workspace.active_view_id();
    let current_pos = active
        .and_then(|id| tab_order.iter().position(|&v| v == id))
        .unwrap_or(0);
    let prev_pos = if current_pos == 0 { tab_order.len() - 1 } else { current_pos - 1 };
    let prev_id = tab_order[prev_pos];
    self.workspace.set_active_view(prev_id);
    self.needs_render = true;
}
```

Note: `tab_order()` borrows workspace immutably and returns a `&[ViewId]`. Copy the IDs
before calling `set_active_view` to avoid borrow conflict:

```rust
let tab_order: Vec<ViewId> = self.workspace.tab_order().to_vec();
```

### Keybinding Wiring in wgpu_client

The wgpu_client main.rs handles winit events and dispatches EditorCommands. Add:

```rust
// In wgpu_client/src/main.rs keyboard handling
KeyCode::W if modifiers.control_key() => {
    adapter.dispatch_command(EditorCommand::CloseTab);
}
KeyCode::Tab if modifiers.control_key() && !modifiers.shift_key() => {
    adapter.dispatch_command(EditorCommand::SwitchTab(0)); // 0 = cycle next
}
KeyCode::Tab if modifiers.control_key() && modifiers.shift_key() => {
    // Need a SwitchTabPrev command or use negative sentinel
    adapter.app.switch_tab_prev();
    // Or: add EditorCommand::SwitchTabPrev to the enum
}
```

Alternatively, add `SwitchTabNext` / `SwitchTabPrev` variants to ora's EditorCommand
mirror to avoid special-casing in the adapter.

### Dot Dirty Indicator in TabBarView

```rust
// In ora/src/views/tab_bar.rs  render_tab()
// Replace current "* {}" formatting with separate dot element:

let dirty_dot = if tab.is_dirty {
    Some(
        TextElement::new("•")
            .size(TAB_FONT_SIZE)
            .color(theme.color(ColorToken::FgMuted))
    )
} else {
    None
};

// Build tab children: [dirty_dot?, title_text, close_button]
```

Also update `get_open_views_info` in app.rs to pass the clean filename (without `*` prefix)
as the title — the `is_dirty` field carries the dirty state separately.

---

## State of the Art

| Old Approach | Current Approach | Change | Impact |
|--------------|------------------|--------|--------|
| HashMap iteration for tab order | Vec<ViewId> tab_order | Add in this phase | Stable tab positions |
| open_document always creates new | Registry check first | Add in this phase | Deduplication |
| No MRU on close | mru_stack Vec | Add in this phase | JetBrains-style focus |
| `* filename` dirty title | `• filename` with dot | Change in this phase | VS Code style |

**Deprecated/outdated in this phase:**
- `can_switch_active()`: Was used in request_open_file for open-file flow. Must NOT be
  used for SwitchTab. The intent is: "do you want to discard unsaved changes before
  replacing this buffer?" That's close-tab concern, not switch-tab.
- `open_file_from_sidebar()`: Currently calls `open_document` (creates new) then
  `create_view`. With Buffer Registry, it needs to: check registry, if existing switch
  to that view; otherwise create normally.

---

## Open Questions

1. **Ctrl+Shift+Tab keybinding on Windows**
   - What we know: winit should deliver Ctrl+Shift+Tab as Tab key + Ctrl+Shift modifiers
   - What's unclear: Whether winit on Windows conflates Tab and Shift+Tab at the event level
   - Recommendation: Test at implementation time; if conflated, check shift modifier explicitly

2. **Middle-click close event delivery**
   - What we know: ora's element system has click handling; middle button may not be wired
   - What's unclear: Whether ora's Div element supports middle-click events
   - Recommendation: Check ora element event system at implementation; fallback is close button only

3. **Tab overflow scroll arrows**
   - What we know: CONTEXT says left/right arrows for overflow (VS Code style)
   - What's unclear: Whether ora's current layout supports horizontal overflow scroll
   - Recommendation: Implement simple clip-and-scroll in TabBarView; defer full scroll arrows
     if ora layout doesn't support horizontal overflow natively

---

## Sources

All findings are from direct codebase inspection (HIGH confidence):

- `core_editor/src/domain/workspace.rs` — Workspace struct, all view/document lifecycle methods
- `core_editor/src/domain/document.rs` — Document, DocumentId, dirty state, title()
- `core_editor/src/view/editor_view.rs` — EditorView, ViewId, per-view state (viewport, carets, selections)
- `core_editor/src/app.rs` — App, dispatch, get_open_views_info, open_file_from_sidebar
- `core_editor/src/view_model/mod.rs` — All Presentation types (Tab, Dialog, Status, etc.)
- `core_editor/src/view_model/builder.rs` — ViewModelBuilder, PendingAction
- `wgpu_client/src/adapter.rs` — CoreEditorAdapter, stub command handlers, SwitchTab/CloseTab stubs
- `ora/src/editor_adapter/types.rs` — Mirror presentation types, EditorCommand (SwitchTab, CloseTab)
- `ora/src/views/tab_bar.rs` — TabBarView renderer, current dirty indicator format

---

## Metadata

**Confidence breakdown:**
- Architecture patterns: HIGH — based on reading actual implementation
- Pitfalls: HIGH — based on reading actual code that will break
- Code examples: MEDIUM — pseudocode showing intent; exact signatures need verification at implementation

**Research date:** 2026-03-26
**Valid until:** 2026-04-26 (stable Rust codebase, no external dependencies)
