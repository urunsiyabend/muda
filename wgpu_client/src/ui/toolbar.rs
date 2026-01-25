//! Toolbar component for IDE actions.
//!
//! A minimal toolbar with icon buttons for common actions.
//! Can be hidden for a more minimal look (like VS Code).

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextAlign, CornerRadii,
    InteractionState, InteractionTracker,
    Space, Radius,
    tokens::ColorPalette,
};

/// Toolbar action types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    NewFile,
    OpenFile,
    SaveFile,
    SaveAll,
    Undo,
    Redo,
    Find,
    Replace,
    ToggleSidebar,
    TogglePanel,
    Settings,
}

/// A toolbar button.
struct ToolbarButton {
    action: ToolbarAction,
    icon: char, // Unicode icon
    tooltip: &'static str,
    interaction: InteractionTracker,
    bounds: Bounds,
}

impl ToolbarButton {
    fn new(action: ToolbarAction, icon: char, tooltip: &'static str) -> Self {
        Self {
            action,
            icon,
            tooltip,
            interaction: InteractionTracker::new(),
            bounds: Bounds::default(),
        }
    }
}

/// The main toolbar component.
pub struct Toolbar {
    buttons: Vec<ToolbarButton>,
    bounds: Bounds,
    palette: ColorPalette,
    visible: bool,
}

impl Toolbar {
    const HEIGHT: f32 = 40.0;
    const BUTTON_SIZE: f32 = 32.0;
    const BUTTON_GAP: f32 = 4.0;
    const PADDING_H: f32 = 8.0;

    pub fn new() -> Self {
        let buttons = vec![
            ToolbarButton::new(ToolbarAction::NewFile, '📄', "New File (Ctrl+N)"),
            ToolbarButton::new(ToolbarAction::OpenFile, '📂', "Open File (Ctrl+O)"),
            ToolbarButton::new(ToolbarAction::SaveFile, '💾', "Save (Ctrl+S)"),
            ToolbarButton::new(ToolbarAction::Undo, '↩', "Undo (Ctrl+Z)"),
            ToolbarButton::new(ToolbarAction::Redo, '↪', "Redo (Ctrl+Y)"),
            ToolbarButton::new(ToolbarAction::Find, '🔍', "Find (Ctrl+F)"),
            ToolbarButton::new(ToolbarAction::ToggleSidebar, '☰', "Toggle Sidebar (Ctrl+B)"),
        ];

        Self {
            buttons,
            bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            visible: false, // Hidden by default like modern IDEs
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn height(&self) -> f32 {
        if self.visible { Self::HEIGHT } else { 0.0 }
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;

        // Layout buttons
        let mut x = bounds.x + Self::PADDING_H;
        let y = bounds.y + (bounds.height - Self::BUTTON_SIZE) / 2.0;

        for button in &mut self.buttons {
            button.bounds = Bounds {
                x,
                y,
                width: Self::BUTTON_SIZE,
                height: Self::BUTTON_SIZE,
            };
            x += Self::BUTTON_SIZE + Self::BUTTON_GAP;
        }
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        if !self.visible {
            return Vec::new();
        }

        let mut rects = Vec::new();

        // Toolbar background
        rects.push(
            StyledRect::new(self.bounds)
                .with_fill(self.palette.bg_secondary)
        );

        // Bottom border
        rects.push(
            StyledRect::new(Bounds {
                x: self.bounds.x,
                y: self.bounds.y + self.bounds.height - 1.0,
                width: self.bounds.width,
                height: 1.0,
            })
            .with_fill(self.palette.border_default)
        );

        // Button backgrounds
        for button in &self.buttons {
            let state = button.interaction.state();
            let bg_color = match state {
                InteractionState::Hovered => self.palette.interactive_hover,
                InteractionState::Pressed => self.palette.interactive_pressed,
                _ => Color::TRANSPARENT,
            };

            if bg_color.a > 0.0 {
                rects.push(
                    StyledRect::new(button.bounds)
                        .with_fill(bg_color)
                        .with_corner_radii(CornerRadii::all(Radius::Sm.px()))
                );
            }
        }

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        if !self.visible {
            return Vec::new();
        }

        let mut texts = Vec::new();

        for button in &self.buttons {
            let text = TextBlock::new(
                button.icon.to_string(),
                button.bounds,
            )
            .with_color(self.palette.fg_primary)
            .with_font_size(18.0)
            .with_align(TextAlign::Center);

            texts.push(text);
        }

        texts
    }

    /// Handle pointer event. Returns the action if a button was clicked.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }

        let mut changed = false;
        for button in &mut self.buttons {
            let was_hovered = button.interaction.is_hovered();
            if point_in_bounds(button.bounds, x, y) {
                button.interaction.on_pointer_enter();
            } else {
                button.interaction.on_pointer_leave();
            }
            if was_hovered != button.interaction.is_hovered() {
                changed = true;
            }
        }
        changed
    }

    pub fn on_click(&mut self, x: f32, y: f32) -> Option<ToolbarAction> {
        if !self.visible {
            return None;
        }

        for button in &self.buttons {
            if point_in_bounds(button.bounds, x, y) {
                return Some(button.action);
            }
        }
        None
    }
}

fn point_in_bounds(bounds: Bounds, x: f32, y: f32) -> bool {
    x >= bounds.x && x < bounds.x + bounds.width &&
    y >= bounds.y && y < bounds.y + bounds.height
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}
