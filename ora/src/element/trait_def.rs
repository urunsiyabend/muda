use crate::animation::transition::{TransitionConfig, TransitionId};
use crate::entity::EntityStorage;
use crate::events::focus::{FocusId, FocusState};
use crate::events::interaction::InteractionState;
use crate::events::mouse::{Hitbox, HitboxId, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use crate::events::dispatch::EventHandlers;
use crate::layout::{compute_flexbox, AvailableSpace, LayoutInput, LayoutOutput};
use crate::style::units::{Rect, Size};
use crate::style::{Color, Style};
use crate::theme::Theme;

/// Opaque handle to layout data produced by the layout engine.
#[derive(Debug, Clone, Copy)]
pub struct LayoutId(pub(crate) usize);

/// GPU rendering command produced during paint phase.
#[derive(Clone)]
pub enum PaintCommand {
    /// Draw a colored rectangle at the given position and size.
    /// Legacy from Phase 1, kept for compatibility.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
    /// Draw a styled rectangle with background, border, border-radius, and shadow.
    StyledRect {
        bounds: Rect,
        style: Style,
    },
    /// Draw text at the given position with the specified style.
    Text {
        buffer: glyphon::Buffer,
        left: f32,
        top: f32,
        bounds: Rect,
        color: Color,
    },
    /// Set scissor rectangle for clipping
    SetScissor {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    /// Reset scissor to full viewport
    ResetScissor,
    /// Marks a boundary between z-layers (e.g., between Stack children).
    /// The renderer flushes rects and text at each boundary to maintain correct z-ordering.
    LayerBoundary,
}

/// Context for computing layout requirements.
/// Provides access to entity storage and layout state.
pub struct LayoutContext<'a> {
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) next_layout_id: usize,
    pub(crate) window_size: (u32, u32),
    pub(crate) layout_outputs: Vec<LayoutOutput>,
    pub(crate) styles: Vec<Style>,
    pub(crate) children_map: Vec<Vec<usize>>,
    pub(crate) intrinsic_sizes: Vec<Option<Size<f32>>>,
    /// Optional reference to TextSystem for text measurement during layout.
    /// When None, text measurement falls back to rough estimation.
    pub(crate) text_system: Option<*mut crate::rendering::TextSystem>,
}

impl<'a> LayoutContext<'a> {
    pub(crate) fn new(entity_storage: &'a mut EntityStorage, window_size: (u32, u32)) -> Self {
        Self {
            entity_storage,
            next_layout_id: 0,
            window_size,
            layout_outputs: Vec::new(),
            styles: Vec::new(),
            children_map: Vec::new(),
            intrinsic_sizes: Vec::new(),
            text_system: None,
        }
    }

    /// Set the TextSystem reference for text measurement.
    pub(crate) fn set_text_system(&mut self, text_system: *mut crate::rendering::TextSystem) {
        self.text_system = Some(text_system);
    }

    /// Measure text and return the Buffer (for reuse during paint) and measured size.
    /// The Buffer must be stored by the element and passed to paint_text during paint phase.
    pub fn measure_text(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        max_width: Option<f32>,
    ) -> (glyphon::Buffer, Size<f32>) {
        if let Some(text_system_ptr) = self.text_system {
            // SAFETY: The text_system pointer is valid for the duration of the layout phase.
            // It's set by OraWindow::render before calling request_layout.
            unsafe {
                (*text_system_ptr).measure_text(text, font_size, line_height, max_width)
            }
        } else {
            // Fallback: rough estimation when TextSystem is not available
            // This should not happen in production, but provides a safe fallback
            let char_count = text.chars().count();
            let width = if let Some(max_w) = max_width {
                max_w.min(char_count as f32 * font_size * 0.6)
            } else {
                char_count as f32 * font_size * 0.6
            };
            let lines = if let Some(max_w) = max_width {
                ((char_count as f32 * font_size * 0.6) / max_w).ceil().max(1.0)
            } else {
                1.0
            };
            let height = lines * line_height;

            // Create a dummy buffer - this won't be used for rendering
            let mut font_system = glyphon::FontSystem::new();
            let metrics = glyphon::Metrics::new(font_size, line_height);
            let buffer = glyphon::Buffer::new(&mut font_system, metrics);

            (buffer, Size::new(width, height))
        }
    }

    /// Request layout for an element with its style.
    /// Returns a LayoutId that can be used to retrieve computed bounds later.
    pub fn request_layout(&mut self, style: &Style) -> LayoutId {
        let id = LayoutId(self.next_layout_id);
        self.next_layout_id += 1;

        self.styles.push(style.clone());
        self.intrinsic_sizes.push(None);
        self.children_map.push(Vec::new());
        self.layout_outputs.push(LayoutOutput::zero());

        id
    }

    /// Set the intrinsic size for a layout node (e.g., measured text dimensions).
    pub fn set_intrinsic_size(&mut self, id: LayoutId, size: Size<f32>) {
        if id.0 < self.intrinsic_sizes.len() {
            self.intrinsic_sizes[id.0] = Some(size);
        }
    }

    /// Register a parent-child relationship for layout computation.
    pub fn add_child(&mut self, parent: LayoutId, child: LayoutId) {
        if parent.0 < self.children_map.len() {
            self.children_map[parent.0].push(child.0);
        }
    }

    /// Compute layout for all registered nodes using the flexbox algorithm.
    pub fn compute(&mut self) {
        if self.styles.is_empty() {
            return;
        }

        let available = LayoutInput::new(
            AvailableSpace::Definite(self.window_size.0 as f32),
            AvailableSpace::Definite(self.window_size.1 as f32),
        );

        let style_refs: Vec<&Style> = self.styles.iter().collect();
        self.layout_outputs = compute_flexbox(
            &style_refs,
            &self.children_map,
            &self.intrinsic_sizes,
            0, // Root node
            available,
        );
    }

    /// Get the computed bounds for a layout node.
    pub fn bounds(&self, id: LayoutId) -> Rect {
        self.layout_outputs
            .get(id.0)
            .map(|output| output.bounds)
            .unwrap_or_else(Rect::zero)
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
}

/// Context for registering hitboxes and preparing for painting.
/// Called after layout is resolved, before paint.
pub struct PrepaintContext<'a> {
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) window_size: (u32, u32),
    pub(crate) layout_outputs: &'a [LayoutOutput],
    pub(crate) hitboxes: Vec<Hitbox>,
    pub(crate) next_hitbox_id: u64,
    /// Focus IDs of focusable elements in tree order.
    pub(crate) focusable_elements: Vec<FocusId>,
    /// Event handlers registry.
    pub(crate) event_handlers: EventHandlers,
    /// Stack of parent hitboxes for building parent relationships.
    pub(crate) hitbox_stack: Vec<HitboxId>,
}

impl<'a> PrepaintContext<'a> {
    pub(crate) fn new(
        entity_storage: &'a mut EntityStorage,
        window_size: (u32, u32),
        layout_outputs: &'a [LayoutOutput],
    ) -> Self {
        Self {
            entity_storage,
            window_size,
            layout_outputs,
            hitboxes: Vec::new(),
            next_hitbox_id: 0,
            focusable_elements: Vec::new(),
            event_handlers: EventHandlers::new(),
            hitbox_stack: Vec::new(),
        }
    }

    /// Register a hitbox for mouse event routing.
    /// Returns a HitboxId that can be used to identify this element when hit.
    pub fn register_hitbox(&mut self, bounds: Rect, opaque: bool) -> HitboxId {
        let id = HitboxId(self.next_hitbox_id);
        self.next_hitbox_id += 1;

        let hitbox = Hitbox { id, bounds, opaque };
        self.hitboxes.push(hitbox);

        id
    }

    /// Extract the collected hitboxes (for internal use by event loop).
    pub(crate) fn take_hitboxes(&mut self) -> Vec<Hitbox> {
        std::mem::take(&mut self.hitboxes)
    }

    /// Register an element as focusable for tab navigation.
    /// Call during prepaint for any element that can receive keyboard focus.
    pub fn register_focusable(&mut self, focus_id: FocusId) {
        self.focusable_elements.push(focus_id);
    }

    /// Extract collected focusable elements (for internal use by event loop).
    pub(crate) fn take_focusables(&mut self) -> Vec<FocusId> {
        std::mem::take(&mut self.focusable_elements)
    }

    /// Extract event handlers (for internal use by event loop).
    pub(crate) fn take_event_handlers(&mut self) -> EventHandlers {
        std::mem::take(&mut self.event_handlers)
    }

    /// Register a mouse down handler for a hitbox.
    pub fn on_mouse_down(
        &mut self,
        hitbox_id: HitboxId,
        handler: impl FnMut(&MouseDownEvent, &crate::events::dispatch::EventContext) + 'static,
    ) {
        self.event_handlers.register_mouse_down(hitbox_id, Box::new(handler));
    }

    /// Register a mouse up handler for a hitbox.
    pub fn on_mouse_up(
        &mut self,
        hitbox_id: HitboxId,
        handler: impl FnMut(&MouseUpEvent, &crate::events::dispatch::EventContext) + 'static,
    ) {
        self.event_handlers.register_mouse_up(hitbox_id, Box::new(handler));
    }

    /// Register a mouse move handler for a hitbox.
    pub fn on_mouse_move(
        &mut self,
        hitbox_id: HitboxId,
        handler: impl FnMut(&MouseMoveEvent, &crate::events::dispatch::EventContext) + 'static,
    ) {
        self.event_handlers.register_mouse_move(hitbox_id, Box::new(handler));
    }

    /// Push a parent hitbox onto the stack.
    /// Call before processing children to establish parent relationships.
    pub fn push_hitbox_parent(&mut self, parent: HitboxId) {
        self.hitbox_stack.push(parent);
    }

    /// Pop a parent hitbox from the stack.
    /// Call after processing all children.
    pub fn pop_hitbox_parent(&mut self) {
        self.hitbox_stack.pop();
    }

    /// Register a hitbox with parent relationship from the current stack.
    /// This should be called when registering a hitbox to establish parent chain.
    pub fn register_hitbox_with_parent(&mut self, bounds: Rect, opaque: bool) -> HitboxId {
        let id = self.register_hitbox(bounds, opaque);

        // Register parent relationship if there's a parent on the stack
        if let Some(&parent) = self.hitbox_stack.last() {
            self.event_handlers.register_parent(id, parent);
        }

        id
    }

    /// Get the computed bounds for a layout node.
    pub fn bounds(&self, id: LayoutId) -> Rect {
        self.layout_outputs
            .get(id.0)
            .map(|output| output.bounds)
            .unwrap_or_else(Rect::zero)
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
}

/// Context for producing GPU rendering commands.
/// Called last in the lifecycle, produces visual output.
pub struct PaintContext<'a> {
    /// Raw pointer to AppContext for theme access.
    /// SAFETY: Valid for the duration of the paint phase.
    pub(crate) app_context: *const crate::context::AppContext,
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) paint_commands: Vec<PaintCommand>,
    pub(crate) window_size: (u32, u32),
    pub(crate) layout_outputs: &'a [LayoutOutput],
    pub(crate) clip_stack: Vec<Rect>,
    pub(crate) interaction_state: &'a InteractionState,
    pub(crate) focus_state: &'a FocusState,
}

impl<'a> PaintContext<'a> {
    pub(crate) fn new(
        app_context: &'a crate::context::AppContext,
        entity_storage: &'a mut EntityStorage,
        window_size: (u32, u32),
        layout_outputs: &'a [LayoutOutput],
        interaction_state: &'a InteractionState,
        focus_state: &'a FocusState,
    ) -> Self {
        Self {
            app_context: app_context as *const _,
            entity_storage,
            paint_commands: Vec::new(),
            window_size,
            layout_outputs,
            clip_stack: Vec::new(),
            interaction_state,
            focus_state,
        }
    }

    /// Add a rectangle rendering command.
    pub fn paint_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.paint_commands.push(PaintCommand::Rect {
            x,
            y,
            width,
            height,
            color,
        });
    }

    /// Add a styled rectangle rendering command.
    pub fn paint_styled_rect(&mut self, style: &Style, bounds: &Rect) {
        self.paint_commands.push(PaintCommand::StyledRect {
            bounds: *bounds,
            style: style.clone(),
        });
    }

    /// Add a text rendering command.
    /// The buffer should be the same Buffer returned from LayoutContext::measure_text()
    /// to ensure measurement and rendering use the same shaped text.
    pub fn paint_text(
        &mut self,
        buffer: glyphon::Buffer,
        color: &Color,
        bounds: &Rect,
    ) {
        self.paint_commands.push(PaintCommand::Text {
            buffer,
            left: bounds.origin.x,
            top: bounds.origin.y,
            bounds: *bounds,
            color: *color,
        });
    }

    /// Insert a layer boundary marker.
    /// The GPU renderer uses this to flush rects and text between z-layers,
    /// ensuring correct occlusion when overlay elements paint over lower content.
    pub fn push_layer_boundary(&mut self) {
        self.paint_commands.push(PaintCommand::LayerBoundary);
    }

    /// Get the computed bounds for a layout node.
    pub fn bounds(&self, id: LayoutId) -> Rect {
        self.layout_outputs
            .get(id.0)
            .map(|output| output.bounds)
            .unwrap_or_else(Rect::zero)
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }

    /// Access the current theme for color lookups
    pub fn theme(&self) -> &Theme {
        // SAFETY: The app_context pointer is valid for the duration of the paint phase.
        // It's set by event_loop before calling paint and not mutated during paint.
        unsafe { (*self.app_context).theme() }
    }

    // Interaction state queries

    /// Check if a hitbox is currently hovered
    pub fn is_hovered(&self, hitbox_id: HitboxId) -> bool {
        self.interaction_state.is_hovered(hitbox_id)
    }

    /// Check if a hitbox is currently active (mouse pressed)
    pub fn is_active(&self, hitbox_id: HitboxId) -> bool {
        self.interaction_state.is_active(hitbox_id)
    }

    /// Check if an element is keyboard-focused
    pub fn is_focused(&self, focus_id: FocusId) -> bool {
        self.focus_state.focused_id() == Some(focus_id) && self.focus_state.is_keyboard_focused()
    }

    /// Push a clipping rectangle onto the stack
    pub fn push_clip(&mut self, clip_rect: Rect) {
        // Round scissor coordinates to nearest integer (RESEARCH.md Pitfall 3)
        let x = clip_rect.origin.x.round() as u32;
        let y = clip_rect.origin.y.round() as u32;
        let width = clip_rect.size.width.round() as u32;
        let height = clip_rect.size.height.round() as u32;

        self.clip_stack.push(clip_rect);
        self.paint_commands.push(PaintCommand::SetScissor {
            x,
            y,
            width,
            height,
        });
    }

    /// Pop the top clipping rectangle from the stack
    pub fn pop_clip(&mut self) {
        if self.clip_stack.pop().is_some() {
            if let Some(previous) = self.clip_stack.last() {
                // Restore previous scissor
                let x = previous.origin.x.round() as u32;
                let y = previous.origin.y.round() as u32;
                let width = previous.size.width.round() as u32;
                let height = previous.size.height.round() as u32;

                self.paint_commands.push(PaintCommand::SetScissor {
                    x,
                    y,
                    width,
                    height,
                });
            } else {
                // No more clips, reset to full viewport
                self.paint_commands.push(PaintCommand::ResetScissor);
            }
        }
    }

    /// Get or advance the background-color transition for the given element.
    ///
    /// SAFETY: app_context pointer is valid for the duration of the paint phase.
    /// RefCell provides interior mutability — no aliased mutable borrows possible
    /// since paint is single-threaded and no other code borrows the registry.
    pub fn advance_transition_bg(
        &self,
        id: TransitionId,
        target: Color,
        config: &TransitionConfig,
    ) -> Color {
        let app_cx = unsafe { &*self.app_context };
        let mut registry = app_cx.transition_registry.borrow_mut();
        let state = registry.get_or_create(id);
        state.advance_bg(target, config)
    }

    /// Get or advance the opacity transition for the given element.
    pub fn advance_transition_opacity(
        &self,
        id: TransitionId,
        target: f32,
        config: &TransitionConfig,
    ) -> f32 {
        let app_cx = unsafe { &*self.app_context };
        let mut registry = app_cx.transition_registry.borrow_mut();
        let state = registry.get_or_create(id);
        state.advance_opacity(target, config)
    }

    /// Get or advance the text/border color transition for the given element.
    pub fn advance_transition_color(
        &self,
        id: TransitionId,
        target: Color,
        config: &TransitionConfig,
    ) -> Color {
        let app_cx = unsafe { &*self.app_context };
        let mut registry = app_cx.transition_registry.borrow_mut();
        let state = registry.get_or_create(id);
        state.advance_color(target, config)
    }

    /// Get or advance the horizontal position-offset transition for the given element.
    pub fn advance_transition_position_x(
        &self,
        id: TransitionId,
        target: f32,
        config: &TransitionConfig,
    ) -> f32 {
        let app_cx = unsafe { &*self.app_context };
        let mut registry = app_cx.transition_registry.borrow_mut();
        let state = registry.get_or_create(id);
        state.advance_position_x(target, config)
    }

    /// Get or advance the vertical position-offset transition for the given element.
    pub fn advance_transition_position_y(
        &self,
        id: TransitionId,
        target: f32,
        config: &TransitionConfig,
    ) -> f32 {
        let app_cx = unsafe { &*self.app_context };
        let mut registry = app_cx.transition_registry.borrow_mut();
        let state = registry.get_or_create(id);
        state.advance_position_y(target, config)
    }

    /// Extract the collected paint commands.
    pub(crate) fn take_commands(self) -> Vec<PaintCommand> {
        self.paint_commands
    }
}

/// Element trait defining the three-phase rendering lifecycle.
///
/// Elements are low-level rendering primitives that implement:
/// 1. request_layout: Compute layout requirements
/// 2. prepaint: Register hitboxes and prepare for painting
/// 3. paint: Produce GPU rendering commands
///
/// Elements are stateless between frames. State is passed through
/// RequestLayoutState from layout to prepaint to paint.
pub trait Element: 'static {
    /// State persisted between lifecycle phases within a single frame.
    /// Produced during layout, passed to prepaint and paint.
    type RequestLayoutState: 'static;

    /// Compute layout requirements. Called first in the lifecycle.
    /// Returns a LayoutId handle and state for subsequent phases.
    fn request_layout(
        &mut self,
        cx: &mut LayoutContext,
    ) -> (LayoutId, Self::RequestLayoutState);

    /// Register hitboxes and prepare for painting.
    /// Called after layout is resolved, before paint.
    fn prepaint(
        &mut self,
        state: &mut Self::RequestLayoutState,
        cx: &mut PrepaintContext,
    );

    /// Produce GPU rendering commands.
    /// Called last, produces the visual output.
    fn paint(
        &mut self,
        state: &mut Self::RequestLayoutState,
        cx: &mut PaintContext,
    );
}
