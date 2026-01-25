//! Bottom panel system for terminal, output, problems, etc.
//!
//! Provides a resizable panel area at the bottom of the IDE with tabs
//! for different panel types.

use crate::components::Bounds;
use crate::theme::Color;
use crate::design_system::{
    StyledRect, TextBlock, TextAlign, CornerRadii,
    InteractionState, InteractionTracker,
    Space, Radius,
    tokens::ColorPalette,
};

/// Panel types available in the IDE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    Terminal,
    Output,
    Problems,
    Debug,
}

impl PanelKind {
    pub fn label(&self) -> &'static str {
        match self {
            PanelKind::Terminal => "Terminal",
            PanelKind::Output => "Output",
            PanelKind::Problems => "Problems",
            PanelKind::Debug => "Debug",
        }
    }

    pub fn icon(&self) -> char {
        match self {
            PanelKind::Terminal => '▶',
            PanelKind::Output => '📋',
            PanelKind::Problems => '⚠',
            PanelKind::Debug => '🐛',
        }
    }
}

/// Panel tab state.
struct PanelTab {
    kind: PanelKind,
    interaction: InteractionTracker,
    bounds: Bounds,
}

impl PanelTab {
    fn new(kind: PanelKind, is_active: bool) -> Self {
        let mut tab = Self {
            kind,
            interaction: InteractionTracker::new(),
            bounds: Bounds::default(),
        };
        tab.interaction.set_selected(is_active);
        tab
    }
}

/// Panel content line.
#[derive(Clone, Debug)]
pub struct PanelLine {
    pub text: String,
    pub style: PanelLineStyle,
}

/// Line styling for panel content.
#[derive(Clone, Copy, Debug, Default)]
pub enum PanelLineStyle {
    #[default]
    Normal,
    Error,
    Warning,
    Success,
    Muted,
}

/// The panel manager handles the bottom panel area.
pub struct PanelManager {
    tabs: Vec<PanelTab>,
    active_panel: PanelKind,
    bounds: Bounds,
    content_bounds: Bounds,
    palette: ColorPalette,
    visible: bool,
    height: f32,
    min_height: f32,
    max_height: f32,
    is_resizing: bool,
    resize_start_y: f32,
    resize_start_height: f32,
    // Content for each panel
    terminal_lines: Vec<PanelLine>,
    output_lines: Vec<PanelLine>,
    problems_lines: Vec<PanelLine>,
    scroll_offset: f32,
}

impl PanelManager {
    const TAB_HEIGHT: f32 = 32.0;
    const TAB_PADDING_H: f32 = 12.0;
    const RESIZE_HANDLE_HEIGHT: f32 = 4.0;
    const LINE_HEIGHT: f32 = 20.0;
    const CONTENT_PADDING: f32 = 8.0;

    pub fn new() -> Self {
        let tabs = vec![
            PanelTab::new(PanelKind::Terminal, true),
            PanelTab::new(PanelKind::Output, false),
            PanelTab::new(PanelKind::Problems, false),
        ];

        Self {
            tabs,
            active_panel: PanelKind::Terminal,
            bounds: Bounds::default(),
            content_bounds: Bounds::default(),
            palette: ColorPalette::dark(),
            visible: false,
            height: 200.0,
            min_height: 100.0,
            max_height: 500.0,
            is_resizing: false,
            resize_start_y: 0.0,
            resize_start_height: 0.0,
            terminal_lines: Vec::new(),
            output_lines: Vec::new(),
            problems_lines: Vec::new(),
            scroll_offset: 0.0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn height(&self) -> f32 {
        if self.visible { self.height } else { 0.0 }
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.layout_tabs();

        // Content area
        self.content_bounds = Bounds {
            x: bounds.x,
            y: bounds.y + Self::TAB_HEIGHT + Self::RESIZE_HANDLE_HEIGHT,
            width: bounds.width,
            height: bounds.height - Self::TAB_HEIGHT - Self::RESIZE_HANDLE_HEIGHT,
        };
    }

    fn layout_tabs(&mut self) {
        let mut x = self.bounds.x;
        let y = self.bounds.y + Self::RESIZE_HANDLE_HEIGHT;

        for tab in &mut self.tabs {
            let label_width = tab.kind.label().len() as f32 * 8.0;
            let tab_width = Self::TAB_PADDING_H * 2.0 + label_width + 20.0; // icon space

            tab.bounds = Bounds {
                x,
                y,
                width: tab_width,
                height: Self::TAB_HEIGHT,
            };

            x += tab_width;
        }
    }

    pub fn set_active(&mut self, kind: PanelKind) {
        self.active_panel = kind;
        for tab in &mut self.tabs {
            tab.interaction.set_selected(tab.kind == kind);
        }
        self.scroll_offset = 0.0;
    }

    /// Add a line to a panel.
    pub fn push_line(&mut self, kind: PanelKind, line: PanelLine) {
        let lines = match kind {
            PanelKind::Terminal => &mut self.terminal_lines,
            PanelKind::Output => &mut self.output_lines,
            PanelKind::Problems => &mut self.problems_lines,
            PanelKind::Debug => return,
        };
        lines.push(line);

        // Keep only last 1000 lines
        if lines.len() > 1000 {
            lines.remove(0);
        }
    }

    /// Clear panel content.
    pub fn clear(&mut self, kind: PanelKind) {
        match kind {
            PanelKind::Terminal => self.terminal_lines.clear(),
            PanelKind::Output => self.output_lines.clear(),
            PanelKind::Problems => self.problems_lines.clear(),
            PanelKind::Debug => {},
        }
    }

    fn current_lines(&self) -> &Vec<PanelLine> {
        match self.active_panel {
            PanelKind::Terminal => &self.terminal_lines,
            PanelKind::Output => &self.output_lines,
            PanelKind::Problems => &self.problems_lines,
            PanelKind::Debug => &self.terminal_lines, // fallback
        }
    }

    pub fn build_rects(&self) -> Vec<StyledRect> {
        if !self.visible {
            return Vec::new();
        }

        let mut rects = Vec::new();

        // Panel background
        rects.push(
            StyledRect::new(self.bounds)
                .with_fill(self.palette.bg_secondary)
        );

        // Resize handle
        rects.push(
            StyledRect::new(Bounds {
                x: self.bounds.x,
                y: self.bounds.y,
                width: self.bounds.width,
                height: Self::RESIZE_HANDLE_HEIGHT,
            })
            .with_fill(self.palette.border_default)
        );

        // Tab bar background
        rects.push(
            StyledRect::new(Bounds {
                x: self.bounds.x,
                y: self.bounds.y + Self::RESIZE_HANDLE_HEIGHT,
                width: self.bounds.width,
                height: Self::TAB_HEIGHT,
            })
            .with_fill(self.palette.bg_tertiary)
        );

        // Tab backgrounds
        for tab in &self.tabs {
            let is_active = tab.kind == self.active_panel;
            let is_hovered = tab.interaction.is_hovered();

            let bg_color = if is_active {
                self.palette.bg_secondary
            } else if is_hovered {
                self.palette.interactive_hover
            } else {
                Color::TRANSPARENT
            };

            if bg_color.a > 0.0 || is_active {
                rects.push(
                    StyledRect::new(tab.bounds)
                        .with_fill(bg_color)
                );
            }

            // Active indicator
            if is_active {
                rects.push(
                    StyledRect::new(Bounds {
                        x: tab.bounds.x,
                        y: tab.bounds.y,
                        width: tab.bounds.width,
                        height: 2.0,
                    })
                    .with_fill(self.palette.accent_primary)
                );
            }
        }

        // Content area background
        rects.push(
            StyledRect::new(self.content_bounds)
                .with_fill(self.palette.bg_primary)
        );

        rects
    }

    pub fn build_texts(&self) -> Vec<TextBlock> {
        if !self.visible {
            return Vec::new();
        }

        let mut texts = Vec::new();

        // Tab labels
        for tab in &self.tabs {
            let is_active = tab.kind == self.active_panel;

            let fg_color = if is_active {
                self.palette.fg_primary
            } else {
                self.palette.fg_secondary
            };

            texts.push(
                TextBlock::new(tab.kind.label(), tab.bounds)
                    .with_color(fg_color)
                    .with_font_size(12.0)
                    .with_align(TextAlign::Center)
            );
        }

        // Content lines
        let lines = self.current_lines();
        let visible_lines = ((self.content_bounds.height - Self::CONTENT_PADDING * 2.0) / Self::LINE_HEIGHT) as usize;
        let start_line = (self.scroll_offset / Self::LINE_HEIGHT) as usize;

        for (i, line) in lines.iter().skip(start_line).take(visible_lines).enumerate() {
            let y = self.content_bounds.y + Self::CONTENT_PADDING + i as f32 * Self::LINE_HEIGHT;

            let color = match line.style {
                PanelLineStyle::Normal => self.palette.fg_primary,
                PanelLineStyle::Error => self.palette.error,
                PanelLineStyle::Warning => self.palette.warning,
                PanelLineStyle::Success => self.palette.success,
                PanelLineStyle::Muted => self.palette.fg_muted,
            };

            texts.push(
                TextBlock::new(&line.text, Bounds {
                    x: self.content_bounds.x + Self::CONTENT_PADDING,
                    y,
                    width: self.content_bounds.width - Self::CONTENT_PADDING * 2.0,
                    height: Self::LINE_HEIGHT,
                })
                .with_color(color)
                .with_font_size(13.0)
            );
        }

        texts
    }

    /// Handle pointer move. Returns true if redraw needed.
    pub fn on_pointer_move(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }

        // Handle resize dragging
        if self.is_resizing {
            let delta = self.resize_start_y - y;
            self.height = (self.resize_start_height + delta).clamp(self.min_height, self.max_height);
            return true;
        }

        // Check tab hover
        let mut changed = false;
        for tab in &mut self.tabs {
            let in_tab = x >= tab.bounds.x && x < tab.bounds.x + tab.bounds.width &&
                        y >= tab.bounds.y && y < tab.bounds.y + tab.bounds.height;

            if in_tab && !tab.interaction.is_hovered() {
                tab.interaction.on_pointer_enter();
                changed = true;
            } else if !in_tab && tab.interaction.is_hovered() {
                tab.interaction.on_pointer_leave();
                changed = true;
            }
        }

        changed
    }

    /// Check if pointer is on resize handle.
    pub fn is_on_resize_handle(&self, x: f32, y: f32) -> bool {
        self.visible &&
        x >= self.bounds.x && x < self.bounds.x + self.bounds.width &&
        y >= self.bounds.y && y < self.bounds.y + Self::RESIZE_HANDLE_HEIGHT + 4.0
    }

    /// Start resizing.
    pub fn start_resize(&mut self, y: f32) {
        self.is_resizing = true;
        self.resize_start_y = y;
        self.resize_start_height = self.height;
    }

    /// Stop resizing.
    pub fn stop_resize(&mut self) {
        self.is_resizing = false;
    }

    /// Handle click. Returns the clicked panel kind if a tab was clicked.
    pub fn on_click(&mut self, x: f32, y: f32) -> Option<PanelKind> {
        if !self.visible {
            return None;
        }

        // Find clicked tab first
        let clicked_kind = self.tabs.iter()
            .find(|tab| {
                x >= tab.bounds.x && x < tab.bounds.x + tab.bounds.width &&
                y >= tab.bounds.y && y < tab.bounds.y + tab.bounds.height
            })
            .map(|tab| tab.kind);

        // Then activate it
        if let Some(kind) = clicked_kind {
            self.set_active(kind);
        }

        clicked_kind
    }

    /// Handle scroll in content area.
    pub fn scroll(&mut self, delta: f32) {
        let lines = self.current_lines();
        let max_scroll = ((lines.len() as f32 * Self::LINE_HEIGHT) - self.content_bounds.height + Self::CONTENT_PADDING * 2.0).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
    }
}

impl Default for PanelManager {
    fn default() -> Self {
        Self::new()
    }
}
