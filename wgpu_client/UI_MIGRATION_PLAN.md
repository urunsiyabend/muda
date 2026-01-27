# UI Migration Plan: Professional Modern IDE

## Executive Summary

The wgpu_client has a comprehensive design system and UI layer already implemented, but not rendered. The goal is to wire these new components into the renderer while maintaining stability.

### Current State
- **Legacy Components** (`components/`): Being rendered - TextArea, Gutter, Sidebar, TabBar, StatusBar, Caret, Dialog
- **New UI Layer** (`ui/`): NOT rendered - CommandPalette, EditorTabs, FileTree, PanelManager, StatusLine, AppLayout
- **Design System** (`design_system/`): Ready - StyledRect, TextBlock, StyledRectRenderer, tokens, animation

### The Gap
The new UI components produce `StyledRect` and `TextBlock` primitives, but the renderer doesn't consume them. The `StyledRectRenderer` exists but isn't wired into the render loop.

---

## Migration Phases

### Phase 1: Foundation - Wire StyledRect Rendering + Command Palette
**Goal**: Establish the primitive rendering pipeline and show a working command palette.

**Tasks**:
1. Add `StyledRectRenderer` to `GpuRenderer`
2. Create a shared `UITextRenderer` component for rendering `TextBlock` arrays
3. Add command palette rendering pass (rects + text) after the dialog pass
4. Test: Ctrl+Shift+P should show a professional command palette overlay

**Files to Modify**:
- `renderer/mod.rs` - Add StyledRectRenderer, UITextRenderer, command palette render pass

**Estimate**: Core infrastructure + command palette working

---

### Phase 2: Modern Tab Bar with Close Buttons
**Goal**: Replace basic tab bar with professional tabs (close buttons, dirty indicators, hover states).

**Tasks**:
1. Add `ui::EditorTabs` to `GpuRenderer`
2. Replace legacy `TabBar` rendering with `EditorTabs.build_rects()` + `build_texts()`
3. Wire pointer events in `app.rs` for tab hover/close
4. Keep legacy `TabBar` component for now (data source only)

**Files to Modify**:
- `renderer/mod.rs` - Switch tab rendering
- `app.rs` - Route tab bar mouse events to EditorTabs

---

### Phase 3: Bottom Panel System
**Goal**: Add terminal/output/problems panels at the bottom.

**Tasks**:
1. Add `ui::PanelManager` to `GpuRenderer`
2. Connect panel visibility to `AppLayout` config
3. Add panel rendering pass (between editor and status bar)
4. Wire Ctrl+J toggle in `app.rs`
5. Add resize handle interaction

**Files to Modify**:
- `renderer/mod.rs` - Add PanelManager, render panels
- `app.rs` - Handle panel toggle, resize events
- `ui/app_layout.rs` - Ensure bottom_panel calculation is used

---

### Phase 4: Professional Status Line
**Goal**: Replace basic status bar with VS Code-like status line (git branch, diagnostics, encoding, etc.).

**Tasks**:
1. Add `ui::StatusLine` to `GpuRenderer`
2. Replace legacy `StatusBar` rendering
3. Update status data from editor model (cursor position, language, etc.)

**Files to Modify**:
- `renderer/mod.rs` - Switch to StatusLine
- May need updates to StatusPresentation in core_editor

---

### Phase 5: Enhanced File Tree (Sidebar)
**Goal**: Improve sidebar with proper tree affordances (chevrons, icons, indent guides).

**Tasks**:
1. Add `ui::FileTree` component
2. Replace legacy `SidebarComponent` rendering for the file list
3. Wire expand/collapse chevron clicks
4. Add hover/selection states

**Files to Modify**:
- `renderer/mod.rs` - Use FileTree for sidebar content
- `app.rs` - Handle chevron clicks for expand/collapse

---

## Implementation Details

### Shared UITextRenderer

We need a component that can render an array of `TextBlock` using glyphon. This will be shared by all new UI components.

```rust
pub struct UITextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffers: Vec<Buffer>, // Reusable buffer pool
}

impl UITextRenderer {
    pub fn prepare(&mut self, device, queue, texts: &[TextBlock], scale_factor, screen_size);
    pub fn render(&self, pass: &mut RenderPass);
}
```

### Render Order (Updated)

```
1. Clear pass (background)
2. Sidebar background (if visible)
3. Tab bar background (EditorTabs.build_rects)
4. Gutter background
5. Current line highlight
6. Selection backgrounds
7. Bottom panel background (PanelManager.build_rects)
8. Status line background (StatusLine.build_rects)
9. Caret
10. Main text pass:
    - Sidebar text
    - Tab text (EditorTabs.build_texts)
    - Gutter text
    - Editor text
    - Panel text
    - Status text
11. Command palette overlay (if visible)
12. Dialog overlay (if visible)
```

### Key Architectural Decisions

1. **Shared Primitives**: All new UI components output `Vec<StyledRect>` and `Vec<TextBlock>`, rendered by shared renderers.

2. **Legacy Coexistence**: Legacy components continue working for TextArea, Gutter, Caret (core editing). New UI layer handles chrome.

3. **Layout Flow**: `AppLayout.calculate()` provides bounds, passed to each component's `set_bounds()` method.

4. **Event Routing**: `LayoutRegion` from hit_test determines which component handles events.

---

## Risk Mitigation

1. **Text Editing Stability**: Core TextArea/Gutter/Caret remain unchanged in Phase 1-2.

2. **Incremental Rollout**: Each phase is independent and can be merged separately.

3. **Fallback**: Legacy components remain functional if issues arise.

---

## Success Criteria

- [ ] Command palette renders with backdrop, shadows, animations
- [ ] Tab bar has close buttons, dirty indicators, hover states
- [ ] Bottom panel can be toggled (Ctrl+J) with tabs
- [ ] Status line shows git branch, cursor pos, language, encoding
- [ ] File tree has expand/collapse chevrons, file icons
- [ ] No regressions in text editing (caret, selection, scrolling)
