use crate::entity::EntityStorage;

/// Opaque handle to layout data produced by the layout engine.
/// The real layout engine will be implemented in Phase 2.
#[derive(Debug, Clone, Copy)]
pub struct LayoutId(pub(crate) usize);

/// GPU rendering command produced during paint phase.
#[derive(Debug, Clone)]
pub enum PaintCommand {
    /// Draw a colored rectangle at the given position and size.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
}

/// Context for computing layout requirements.
/// Provides access to entity storage and layout state.
pub struct LayoutContext<'a> {
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) next_layout_id: usize,
    pub(crate) window_size: (u32, u32),
}

impl<'a> LayoutContext<'a> {
    pub(crate) fn new(entity_storage: &'a mut EntityStorage, window_size: (u32, u32)) -> Self {
        Self {
            entity_storage,
            next_layout_id: 0,
            window_size,
        }
    }

    /// Request layout for an element.
    /// Real layout constraints will be stored with these IDs in Phase 2.
    pub fn request_layout(&mut self) -> LayoutId {
        let id = LayoutId(self.next_layout_id);
        self.next_layout_id += 1;
        id
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
}

impl<'a> PrepaintContext<'a> {
    pub(crate) fn new(entity_storage: &'a mut EntityStorage, window_size: (u32, u32)) -> Self {
        Self {
            entity_storage,
            window_size,
        }
    }

    /// Get the current window size.
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
}

/// Context for producing GPU rendering commands.
/// Called last in the lifecycle, produces visual output.
pub struct PaintContext<'a> {
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) paint_commands: Vec<PaintCommand>,
    pub(crate) window_size: (u32, u32),
}

impl<'a> PaintContext<'a> {
    pub(crate) fn new(entity_storage: &'a mut EntityStorage, window_size: (u32, u32)) -> Self {
        Self {
            entity_storage,
            paint_commands: Vec::new(),
            window_size,
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

    /// Get the current window size.
    pub fn window_size(&self) -> (u32, u32) {
        self.window_size
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
