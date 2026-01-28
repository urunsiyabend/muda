use crate::entity::EntityStorage;
use crate::layout::{compute_flexbox, AvailableSpace, LayoutInput, LayoutOutput};
use crate::style::units::{Rect, Size};
use crate::style::Style;

/// Opaque handle to layout data produced by the layout engine.
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
    pub(crate) layout_outputs: Vec<LayoutOutput>,
    pub(crate) styles: Vec<Style>,
    pub(crate) children_map: Vec<Vec<usize>>,
    pub(crate) intrinsic_sizes: Vec<Option<Size<f32>>>,
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
        }
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
    pub(crate) entity_storage: &'a mut EntityStorage,
    pub(crate) paint_commands: Vec<PaintCommand>,
    pub(crate) window_size: (u32, u32),
    pub(crate) layout_outputs: &'a [LayoutOutput],
}

impl<'a> PaintContext<'a> {
    pub(crate) fn new(
        entity_storage: &'a mut EntityStorage,
        window_size: (u32, u32),
        layout_outputs: &'a [LayoutOutput],
    ) -> Self {
        Self {
            entity_storage,
            paint_commands: Vec::new(),
            window_size,
            layout_outputs,
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
