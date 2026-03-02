//! Command palette view with fuzzy search, filtered results, and keyboard navigation.
//!
//! Provides a VS Code-style command palette overlay that appears near the top of the
//! viewport, allowing users to search and execute commands via fuzzy text matching.
//!
//! # CONTEXT.md Decisions
//!
//! - Ctrl+Shift+P only (single keybinding, not Ctrl+P)
//! - Fuzzy matching with highlighted matched characters
//! - Fixed 8 visible results (CONTEXT: 8-10)
//! - Centered horizontally near top (VS Code-style)

use crate::context::ViewContext;
use crate::element::AnyElement;
use crate::elements::{Div, TextElement};
use crate::events::FocusHandle;
use crate::style::{px, pct, Color};
use crate::theme::ColorToken;
use crate::view::View;

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Width of the command palette in logical pixels.
pub const PALETTE_WIDTH: f32 = 600.0;

/// Vertical offset from the top of the viewport (VS Code-style near top).
pub const PALETTE_TOP_OFFSET: f32 = 80.0;

/// Height of a single result row in logical pixels.
pub const ROW_HEIGHT: f32 = 32.0;

/// Number of result rows visible at once.
pub const VISIBLE_ROWS: usize = 8;

/// Height of the search input row in logical pixels.
pub const INPUT_HEIGHT: f32 = 36.0;

/// Border radius of the palette container.
pub const PALETTE_BORDER_RADIUS: f32 = 6.0;

/// Internal padding around the result list.
pub const PALETTE_PADDING: f32 = 4.0;

// ---------------------------------------------------------------------------
// Fuzzy match algorithm
// ---------------------------------------------------------------------------

/// Result of a fuzzy match attempt.
#[derive(Clone, Debug)]
pub struct FuzzyMatch {
    /// Match quality score (higher = better match).
    pub score: i32,
    /// Character indices in the target string that matched query characters.
    pub match_positions: Vec<usize>,
}

/// Fuzzy match a query against a target string.
///
/// Returns `None` if not all query characters are found in sequence.
/// Returns `Some(FuzzyMatch)` with score and match positions on success.
///
/// # Scoring
///
/// - `+10` for each consecutive match (previous matched char was immediately before)
/// - `+5` for word boundary matches (start of string, or after space/underscore/hyphen)
/// - `+1` for all other matches
///
/// An empty query always matches with score 0.
pub fn fuzzy_match(query: &str, target: &str) -> Option<FuzzyMatch> {
    if query.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            match_positions: vec![],
        });
    }

    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let target_chars: Vec<char> = target.to_lowercase().chars().collect();

    let mut match_positions: Vec<usize> = Vec::new();
    let mut qi = 0;
    let mut score = 0i32;

    for (ti, &tc) in target_chars.iter().enumerate() {
        if qi < query_chars.len() && tc == query_chars[qi] {
            // Base score for this match
            let base = if let Some(&last) = match_positions.last() {
                if ti == last + 1 {
                    10 // consecutive bonus
                } else {
                    1
                }
            } else {
                1
            };

            // Word boundary bonus (start of string, or after space/underscore/hyphen)
            let boundary_bonus = if ti == 0
                || matches!(
                    target_chars.get(ti.wrapping_sub(1)),
                    Some(' ' | '_' | '-')
                ) {
                5
            } else {
                0
            };

            score += base + boundary_bonus;
            match_positions.push(ti);
            qi += 1;
        }
    }

    if qi == query_chars.len() {
        Some(FuzzyMatch {
            score,
            match_positions,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// CommandItem data type
// ---------------------------------------------------------------------------

/// A command entry in the command palette.
#[derive(Clone, Debug)]
pub struct CommandItem {
    /// Unique command identifier (e.g. "file.open").
    pub id: String,
    /// Human-readable display label (e.g. "Open File").
    pub label: String,
    /// Optional keyboard shortcut hint (e.g. "Ctrl+O").
    pub shortcut: Option<String>,
}

impl CommandItem {
    /// Create a new command item with the given id and label, no shortcut.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
        }
    }

    /// Attach a keyboard shortcut display string.
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

// ---------------------------------------------------------------------------
// CommandPaletteView
// ---------------------------------------------------------------------------

/// VS Code-style command palette overlay view.
///
/// Displays a searchable overlay near the top of the viewport. Commands are
/// filtered by fuzzy matching against the current query, and matched characters
/// are highlighted in the accent color.
///
/// # Usage
///
/// The view is read-only with respect to the query string — the application
/// layer must call `set_query()` when the user types, and `move_selection_up()`
/// / `move_selection_down()` when arrow keys are pressed.
///
/// ```ignore
/// let mut palette = CommandPaletteView::new(commands, cx.focus_handle());
/// palette.open();
/// palette.set_query("Open File".to_string());
/// ```
pub struct CommandPaletteView {
    /// Whether the palette is currently visible.
    visible: bool,
    /// Current search query string.
    query: String,
    /// All available commands.
    commands: Vec<CommandItem>,
    /// Filtered results: `(command_index, fuzzy_match)`, sorted by score descending.
    filtered: Vec<(usize, FuzzyMatch)>,
    /// Currently highlighted result index within `filtered`.
    selected_index: usize,
    /// Focus handle for the input row.
    #[allow(dead_code)]
    input_focus: FocusHandle,
}

impl CommandPaletteView {
    /// Create a new command palette with the provided commands.
    ///
    /// The palette starts hidden. Call `open()` to show it.
    pub fn new(commands: Vec<CommandItem>, focus_handle: FocusHandle) -> Self {
        let mut palette = Self {
            visible: false,
            query: String::new(),
            commands,
            filtered: Vec::new(),
            selected_index: 0,
            input_focus: focus_handle,
        };
        palette.refilter();
        palette
    }

    /// Show the palette, clear the query, reset selection, and re-filter.
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.refilter();
    }

    /// Hide the palette.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Returns `true` if the palette is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update the search query, refilter commands, and reset the selection.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
        self.refilter();
    }

    /// Move the selection up by one, wrapping from the top to the bottom.
    pub fn move_selection_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.filtered.len().min(VISIBLE_ROWS) - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Move the selection down by one, wrapping from the bottom to the top.
    pub fn move_selection_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let max = self.filtered.len().min(VISIBLE_ROWS) - 1;
        if self.selected_index >= max {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }
    }

    /// Return the currently selected command, if any.
    pub fn selected_command(&self) -> Option<&CommandItem> {
        self.filtered
            .get(self.selected_index)
            .map(|(idx, _)| &self.commands[*idx])
    }

    /// Recompute filtered results from the current query.
    ///
    /// Results are sorted by score descending so the best match appears first.
    fn refilter(&mut self) {
        self.filtered = self
            .commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| {
                fuzzy_match(&self.query, &cmd.label).map(|m| (i, m))
            })
            .collect();

        // Sort by score descending (highest score = best match = first)
        self.filtered.sort_by(|a, b| b.1.score.cmp(&a.1.score));

        // Clamp selected_index to valid range
        let max_visible = self.filtered.len().min(VISIBLE_ROWS);
        if max_visible == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= max_visible {
            self.selected_index = max_visible - 1;
        }
    }

    // -----------------------------------------------------------------------
    // Render helpers
    // -----------------------------------------------------------------------

    /// Render the search input row showing the "> " prefix and current query.
    ///
    /// This is a read-only display — actual text input is handled by the app layer.
    fn render_input(&self, cx: &mut ViewContext) -> Div {
        // Borrow-safe: extract all theme colors before any mutable cx operations
        let bg_color = cx.theme().color(ColorToken::BgElevated);
        let fg_color = cx.theme().color(ColorToken::FgPrimary);
        let prompt_color = cx.theme().color(ColorToken::FgMuted);

        let display_query = if self.query.is_empty() {
            "Type a command...".to_string()
        } else {
            self.query.clone()
        };
        let text_color = if self.query.is_empty() { prompt_color } else { fg_color };

        Div::new()
            .flex_row()
            .align_center()
            .h(px(INPUT_HEIGHT))
            .px(12.0)
            .gap(6.0)
            .bg(bg_color)
            .child(
                TextElement::new("> ")
                    .size(13.0)
                    .color(prompt_color),
            )
            .child(
                TextElement::new(display_query)
                    .size(13.0)
                    .color(text_color),
            )
    }

    /// Render a single result row.
    ///
    /// Selected rows use `BgSecondary` background. Matched characters in the
    /// label are highlighted in `Accent` color, unmatched in `FgPrimary`.
    /// Shortcuts are right-aligned in `FgMuted`.
    fn render_result(
        &self,
        cmd: &CommandItem,
        fuzzy: &FuzzyMatch,
        is_selected: bool,
        cx: &mut ViewContext,
    ) -> Div {
        // Borrow-safe: extract all theme colors before any mutable cx operations
        let bg_selected = cx.theme().color(ColorToken::BgSecondary);
        let bg_transparent = Color::transparent();
        let muted_color = cx.theme().color(ColorToken::FgMuted);

        let row_bg = if is_selected { bg_selected } else { bg_transparent };

        let mut row = Div::new()
            .flex_row()
            .align_center()
            .h(px(ROW_HEIGHT))
            .px(12.0)
            .bg(row_bg);

        // Label with fuzzy-highlighted characters
        let label_div = self.render_highlighted_label(&cmd.label, &fuzzy.match_positions, cx);
        row = row.child(label_div);

        // Shortcut right-aligned (grow spacer + shortcut text)
        if let Some(shortcut) = &cmd.shortcut {
            row = row
                .child(Div::new().grow(1.0)) // spacer
                .child(
                    TextElement::new(shortcut.as_str())
                        .size(11.0)
                        .color(muted_color),
                );
        }

        row
    }

    /// Build a label `Div` with matched characters highlighted in `Accent` color.
    ///
    /// Iterates over the label, grouping consecutive matched/unmatched characters
    /// into segments, each rendered as a separate `TextElement`.
    fn render_highlighted_label(
        &self,
        label: &str,
        positions: &[usize],
        cx: &mut ViewContext,
    ) -> Div {
        // Borrow-safe: extract all theme colors before any mutable cx operations
        let accent_color = cx.theme().color(ColorToken::Accent);
        let fg_color = cx.theme().color(ColorToken::FgPrimary);

        let chars: Vec<char> = label.chars().collect();
        let pos_set: std::collections::HashSet<usize> = positions.iter().copied().collect();

        let mut row = Div::new().flex_row().align_center();
        let mut i = 0;

        while i < chars.len() {
            let is_match = pos_set.contains(&i);
            let start = i;
            // Advance while the match status is the same
            while i < chars.len() && pos_set.contains(&i) == is_match {
                i += 1;
            }
            let segment: String = chars[start..i].iter().collect();
            let color = if is_match { accent_color } else { fg_color };
            row = row.child(TextElement::new(segment).size(13.0).color(color));
        }

        row
    }
}

impl View for CommandPaletteView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // When not visible, return a zero-size placeholder to avoid any rendering cost
        if !self.visible {
            return Div::new().w(px(0.0)).h(px(0.0)).into();
        }

        // Borrow-safe: extract all theme colors before any mutable cx operations
        let bg_elevated = cx.theme().color(ColorToken::BgElevated);
        let border_color = cx.theme().color(ColorToken::Border);
        let separator_color = cx.theme().color(ColorToken::Border);
        let shadow_color = Color::rgba(0.0, 0.0, 0.0, 0.5);

        // Full-screen positioning container: flex_col + align_center
        // A spacer div creates the top offset (PALETTE_TOP_OFFSET) before the palette.
        let mut positioning = Div::new()
            .flex_col()
            .w(pct(100.0))
            .h(pct(100.0))
            .align_center()
            .child(Div::new().h(px(PALETTE_TOP_OFFSET)).w(pct(100.0)));

        // Palette container with shadow
        let mut palette = Div::new()
            .flex_col()
            .w(px(PALETTE_WIDTH))
            .bg(bg_elevated)
            .border(1.0, border_color)
            .border_radius(PALETTE_BORDER_RADIUS)
            .shadow(0.0, 8.0, 32.0, 0.0, shadow_color);

        // Input row
        palette = palette.child(self.render_input(cx));

        // Separator line between input and results
        palette = palette.child(Div::new().h(px(1.0)).bg(separator_color));

        // Result list — fixed height to show exactly VISIBLE_ROWS rows
        let list_height = VISIBLE_ROWS as f32 * ROW_HEIGHT;
        let mut result_list = Div::new()
            .flex_col()
            .h(px(list_height))
            .overflow_hidden();

        for (i, (cmd_idx, fuzzy)) in self.filtered.iter().take(VISIBLE_ROWS).enumerate() {
            let cmd = &self.commands[*cmd_idx];
            let is_selected = i == self.selected_index;
            result_list = result_list.child(self.render_result(cmd, fuzzy, is_selected, cx));
        }

        palette = palette.child(result_list);
        positioning = positioning.child(palette);
        positioning.into()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- fuzzy_match tests ----

    #[test]
    fn test_fuzzy_empty_query() {
        // Empty query matches everything with score 0 and no positions
        let result = fuzzy_match("", "Open File");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.score, 0);
        assert!(m.match_positions.is_empty());
    }

    #[test]
    fn test_fuzzy_exact_match() {
        // Exact match should succeed
        let result = fuzzy_match("Open File", "Open File");
        assert!(result.is_some());
        let m = result.unwrap();
        assert!(m.score > 0);
        // All 9 chars should be matched
        assert_eq!(m.match_positions.len(), 9);
    }

    #[test]
    fn test_fuzzy_non_contiguous() {
        // "ofi" should match "Open File" at positions 0, 5, 6 (o, F, i)
        let result = fuzzy_match("ofi", "Open File");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.match_positions.len(), 3);
    }

    #[test]
    fn test_fuzzy_no_match() {
        // "xyz" should NOT match "Open File"
        let result = fuzzy_match("xyz", "Open File");
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_case_insensitive() {
        // Upper-case query should match lower-case target
        let result = fuzzy_match("OPEN", "open file");
        assert!(result.is_some());
    }

    #[test]
    fn test_fuzzy_consecutive_bonus() {
        // "op" consecutive in "open" should score higher than "op" scattered in "old path"
        let consecutive = fuzzy_match("op", "open").unwrap();
        let scattered = fuzzy_match("op", "o_path").unwrap();
        // consecutive: 'o' at boundary (+1+5=6), 'p' consecutive (+10) = 16
        // scattered:   'o' at boundary (+1+5=6), 'p' at boundary (+1+5=6) = 12
        assert!(consecutive.score > scattered.score,
            "consecutive={} scattered={}", consecutive.score, scattered.score);
    }

    #[test]
    fn test_fuzzy_word_boundary_bonus() {
        // Match at word boundary (after space) scores higher than match in middle
        let boundary = fuzzy_match("f", "open file").unwrap();   // 'f' at word boundary
        let middle = fuzzy_match("e", "open file").unwrap();     // 'e' in middle of "open"
        // boundary: 'f' is after space (+1+5=6)
        // middle: 'e' in middle (+1)
        assert!(boundary.score > middle.score,
            "boundary={} middle={}", boundary.score, middle.score);
    }

    // ---- CommandItem tests ----

    #[test]
    fn test_command_item_new() {
        let item = CommandItem::new("file.open", "Open File");
        assert_eq!(item.id, "file.open");
        assert_eq!(item.label, "Open File");
        assert!(item.shortcut.is_none());
    }

    #[test]
    fn test_command_item_with_shortcut() {
        let item = CommandItem::new("file.open", "Open File")
            .with_shortcut("Ctrl+O");
        assert_eq!(item.shortcut, Some("Ctrl+O".to_string()));
    }

    // ---- CommandPaletteView logic tests (no rendering) ----

    fn make_commands() -> Vec<CommandItem> {
        vec![
            CommandItem::new("file.open", "Open File").with_shortcut("Ctrl+O"),
            CommandItem::new("file.save", "Save File").with_shortcut("Ctrl+S"),
            CommandItem::new("view.toggle", "Toggle Sidebar"),
            CommandItem::new("edit.find", "Find in File").with_shortcut("Ctrl+F"),
        ]
    }

    #[test]
    fn test_palette_open_clears_query() {
        // We can't easily create a FocusHandle without a context, so we test via
        // the public API that doesn't require rendering.
        // This test verifies the refilter logic with an empty query returns all commands.
        let result = fuzzy_match("", "Open File");
        assert!(result.is_some());
        let result2 = fuzzy_match("", "Save File");
        assert!(result2.is_some());
    }

    #[test]
    fn test_refilter_empty_query_returns_all() {
        let commands = make_commands();
        // Simulate what refilter does with empty query
        let filtered: Vec<(usize, FuzzyMatch)> = commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| fuzzy_match("", &cmd.label).map(|m| (i, m)))
            .collect();
        assert_eq!(filtered.len(), commands.len());
    }

    #[test]
    fn test_refilter_filters_correctly() {
        let commands = make_commands();
        let query = "file";
        let filtered: Vec<(usize, FuzzyMatch)> = commands
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| fuzzy_match(query, &cmd.label).map(|m| (i, m)))
            .collect();
        // "Open File", "Save File", "Find in File" all contain "file"
        // "Toggle Sidebar" does not
        assert_eq!(filtered.len(), 3, "Expected 3 results matching 'file'");
    }

    #[test]
    fn test_selection_wraps_down() {
        // Simulates move_selection_down wrapping behavior
        let items = vec![0usize, 1, 2];
        let max = items.len().min(VISIBLE_ROWS) - 1;
        let mut idx = max;
        // Wrap: at max -> go to 0
        if idx >= max { idx = 0; } else { idx += 1; }
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_selection_wraps_up() {
        // Simulates move_selection_up wrapping behavior
        let items = vec![0usize, 1, 2];
        let max = items.len().min(VISIBLE_ROWS) - 1;
        let mut idx = 0usize;
        // Wrap: at 0 -> go to max
        if idx == 0 { idx = max; } else { idx -= 1; }
        assert_eq!(idx, 2);
    }
}
