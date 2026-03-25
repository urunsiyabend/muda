use log::debug;
use std::path::Path;
use tree_sitter::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

use crate::view_model::TextStyle;

/// Maximum file size (in bytes) for real-time incremental syntax parsing.
/// Files larger than this will defer syntax updates to avoid blocking the UI.
/// This is a pragmatic tradeoff used by many professional editors.
const MAX_INCREMENTAL_PARSE_SIZE: usize = 500_000; // 500KB

pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    language: SyntaxLanguage,
    query: Option<Query>,
    /// Cached content length for incremental parsing validation
    last_content_len: usize,
    /// Whether syntax is currently invalidated (needs reparse)
    needs_reparse: bool,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum SyntaxLanguage {
    Rust,
    Python,
    JavaScript,
    Json,
    Markdown,
    #[default]
    Plain,
}

impl SyntaxLanguage {
    pub fn from_extension(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => SyntaxLanguage::Rust,
            Some("py") => SyntaxLanguage::Python,
            Some("js") | Some("jsx") | Some("ts") | Some("tsx") => SyntaxLanguage::JavaScript,
            Some("json") => SyntaxLanguage::Json,
            Some("md") | Some("markdown") => SyntaxLanguage::Markdown,
            _ => SyntaxLanguage::Plain,
        }
    }

    fn get_language(&self) -> Option<tree_sitter::Language> {
        match self {
            SyntaxLanguage::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            SyntaxLanguage::Python => Some(tree_sitter_python::LANGUAGE.into()),
            SyntaxLanguage::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
            SyntaxLanguage::Json => Some(tree_sitter_json::LANGUAGE.into()),
            SyntaxLanguage::Markdown => Some(tree_sitter_md::LANGUAGE.into()),
            SyntaxLanguage::Plain => None,
        }
    }

    fn get_highlight_query(&self) -> &str {
        match self {
            SyntaxLanguage::Rust => RUST_HIGHLIGHTS,
            SyntaxLanguage::Python => PYTHON_HIGHLIGHTS,
            SyntaxLanguage::JavaScript => JS_HIGHLIGHTS,
            SyntaxLanguage::Json => JSON_HIGHLIGHTS,
            SyntaxLanguage::Markdown => MD_HIGHLIGHTS,
            SyntaxLanguage::Plain => "",
        }
    }
}

#[derive(Clone, Debug)]
pub struct HighlightSpan {
    pub start_col: usize,
    pub end_col: usize,
    pub style: TextStyle,
}

impl SyntaxHighlighter {
    /// Returns the language this highlighter is configured for.
    pub fn language(&self) -> SyntaxLanguage {
        self.language
    }

    pub fn new(language: SyntaxLanguage) -> Self {
        let mut parser = Parser::new();
        let mut query = None;

        if let Some(lang) = language.get_language() {
            match parser.set_language(&lang) {
                Ok(()) => {
                    debug!("set_language OK for {:?}", language);

                    let query_str = language.get_highlight_query();
                    if !query_str.is_empty() {
                        match Query::new(&lang, query_str) {
                            Ok(q) => {
                                query = Some(q);
                                debug!("compiled query for {:?}", language);
                            }
                            Err(e) => {
                                debug!("failed to compile query for {:?}: {:?}", language, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("set_language ERROR for {:?}: {:?}", language, e);
                }
            }
        } else {
            debug!("No language set for {:?}", language);
        }

        Self {
            parser,
            tree: None,
            language,
            query,
            last_content_len: 0,
            needs_reparse: false,
        }
    }

    /// Clears the cached tree, forcing a full reparse on next call.
    pub fn invalidate(&mut self) {
        self.tree = None;
        self.last_content_len = 0;
        self.needs_reparse = true;
    }

    /// Returns true if syntax highlighting needs a reparse (was deferred due to large file).
    pub fn needs_reparse(&self) -> bool {
        self.needs_reparse
    }

    /// Marks the highlighter as up-to-date after a background reparse.
    pub fn mark_reparsed(&mut self) {
        self.needs_reparse = false;
    }

    pub fn parse(&mut self, content: &str) {
        if self.language == SyntaxLanguage::Plain {
            self.tree = None;
            self.last_content_len = 0;
            self.needs_reparse = false;
            return;
        }

        // Do a fresh parse each time (for initial load or when no tree exists).
        let new_tree = self.parser.parse(content, None);
        if new_tree.is_none() {
            debug!("parse returned None for {:?}", self.language);
        } else {
            debug!("parse OK for {:?}", self.language);
        }
        self.tree = new_tree;
        self.last_content_len = content.len();
        self.needs_reparse = false;
    }

    /// Parses content from a rope using a callback, avoiding full content clone.
    /// This is more efficient for large files.
    pub fn parse_from_rope(&mut self, rope: &ropey::Rope, old_tree: Option<&Tree>) {
        if self.language == SyntaxLanguage::Plain {
            self.tree = None;
            self.last_content_len = 0;
            return;
        }

        // Tree-sitter's parse_with_options callback needs to return byte slices.
        // Ropey stores text in chunks, so we can return chunk slices directly.
        let new_tree = self.parser.parse_with_options(
            &mut |byte_offset, _position| {
                // Get the chunk at this byte offset
                if byte_offset >= rope.len_bytes() {
                    return "";
                }
                // get_chunk_at_byte returns (&str, char_start, byte_start, byte_end)
                if let Some((chunk_str, _char_start, chunk_byte_start, _chunk_byte_end)) =
                    rope.get_chunk_at_byte(byte_offset)
                {
                    // Calculate offset within the chunk
                    let offset_in_chunk = byte_offset - chunk_byte_start;
                    // Return the remaining part of the chunk from this position
                    return &chunk_str[offset_in_chunk..];
                }
                ""
            },
            old_tree,
            None, // No options
        );

        if new_tree.is_none() {
            debug!("parse_from_rope returned None for {:?}", self.language);
        } else {
            debug!("parse_from_rope OK for {:?}", self.language);
        }
        self.tree = new_tree;
        self.last_content_len = rope.len_bytes();
        self.needs_reparse = false;
    }

    /// Performs an incremental parse after an edit.
    /// This is much faster than full parse for single-character edits.
    ///
    /// # Arguments
    /// * `content` - The new content after the edit
    /// * `edit_start_byte` - Byte offset where the edit started
    /// * `old_end_byte` - Byte offset where the old content ended (for deletions)
    /// * `new_end_byte` - Byte offset where the new content ends (for insertions)
    /// * `start_position` - (row, column) of edit start
    /// * `old_end_position` - (row, column) of old content end
    /// * `new_end_position` - (row, column) of new content end
    pub fn parse_incremental(
        &mut self,
        content: &str,
        edit_start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: (usize, usize),
        old_end_position: (usize, usize),
        new_end_position: (usize, usize),
    ) {
        if self.language == SyntaxLanguage::Plain {
            self.tree = None;
            self.last_content_len = 0;
            return;
        }

        if let Some(ref mut tree) = self.tree {
            // Apply the edit to the tree
            let input_edit = tree_sitter::InputEdit {
                start_byte: edit_start_byte,
                old_end_byte,
                new_end_byte,
                start_position: tree_sitter::Point {
                    row: start_position.0,
                    column: start_position.1,
                },
                old_end_position: tree_sitter::Point {
                    row: old_end_position.0,
                    column: old_end_position.1,
                },
                new_end_position: tree_sitter::Point {
                    row: new_end_position.0,
                    column: new_end_position.1,
                },
            };
            tree.edit(&input_edit);

            // Re-parse with the old tree for incremental parsing
            let new_tree = self.parser.parse(content, Some(tree));
            if new_tree.is_none() {
                debug!("incremental parse returned None for {:?}", self.language);
            } else {
                debug!("incremental parse OK for {:?}", self.language);
            }
            self.tree = new_tree;
        } else {
            // No existing tree, do a full parse
            self.parse(content);
        }
        self.last_content_len = content.len();
    }

    /// Performs an incremental parse from a rope, avoiding full content clone.
    /// For files larger than MAX_INCREMENTAL_PARSE_SIZE, this defers parsing
    /// to avoid blocking the UI (marks needs_reparse for later background update).
    pub fn parse_incremental_from_rope(
        &mut self,
        rope: &ropey::Rope,
        edit_start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: (usize, usize),
        old_end_position: (usize, usize),
        new_end_position: (usize, usize),
    ) {
        if self.language == SyntaxLanguage::Plain {
            self.tree = None;
            self.last_content_len = 0;
            self.needs_reparse = false;
            return;
        }

        // For large files, defer syntax update to avoid blocking UI
        // The existing tree/highlights remain valid until a background reparse
        if rope.len_bytes() > MAX_INCREMENTAL_PARSE_SIZE {
            // Just update the tree with the edit info, but skip the expensive reparse
            if let Some(ref mut tree) = self.tree {
                let input_edit = tree_sitter::InputEdit {
                    start_byte: edit_start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: tree_sitter::Point {
                        row: start_position.0,
                        column: start_position.1,
                    },
                    old_end_position: tree_sitter::Point {
                        row: old_end_position.0,
                        column: old_end_position.1,
                    },
                    new_end_position: tree_sitter::Point {
                        row: new_end_position.0,
                        column: new_end_position.1,
                    },
                };
                tree.edit(&input_edit);
            }
            self.needs_reparse = true;
            self.last_content_len = rope.len_bytes();
            debug!("Deferred syntax update for large file ({} bytes)", rope.len_bytes());
            return;
        }

        if let Some(ref mut tree) = self.tree {
            // Apply the edit to the tree
            let input_edit = tree_sitter::InputEdit {
                start_byte: edit_start_byte,
                old_end_byte,
                new_end_byte,
                start_position: tree_sitter::Point {
                    row: start_position.0,
                    column: start_position.1,
                },
                old_end_position: tree_sitter::Point {
                    row: old_end_position.0,
                    column: old_end_position.1,
                },
                new_end_position: tree_sitter::Point {
                    row: new_end_position.0,
                    column: new_end_position.1,
                },
            };
            tree.edit(&input_edit);

            // Re-parse using rope callback with the edited tree
            let new_tree = self.parser.parse_with_options(
                &mut |byte_offset, _position| {
                    if byte_offset >= rope.len_bytes() {
                        return "";
                    }
                    // get_chunk_at_byte returns (&str, char_start, byte_start, byte_end)
                    if let Some((chunk_str, _char_start, chunk_byte_start, _chunk_byte_end)) =
                        rope.get_chunk_at_byte(byte_offset)
                    {
                        let offset_in_chunk = byte_offset - chunk_byte_start;
                        return &chunk_str[offset_in_chunk..];
                    }
                    ""
                },
                Some(tree),
                None, // No options
            );

            if new_tree.is_none() {
                debug!("incremental parse from rope returned None for {:?}", self.language);
            } else {
                debug!("incremental parse from rope OK for {:?}", self.language);
            }
            self.tree = new_tree;
        } else {
            // No existing tree, do a full parse from rope
            self.parse_from_rope(rope, None);
        }
        self.last_content_len = rope.len_bytes();
        self.needs_reparse = false;
    }

    /// Quick check if we have a valid tree for this content.
    pub fn has_tree(&self) -> bool {
        self.tree.is_some()
    }

    pub fn highlight_line(
        &self,
        content: &str,
        line_idx: usize,
        line_start_byte: usize,
        line_text: &str,
    ) -> Vec<HighlightSpan> {
        let spans = Vec::new();

        if self.language == SyntaxLanguage::Plain {
            return spans;
        }

        let tree = match &self.tree {
            Some(t) => t,
            None => {
                debug!(
                    "highlight_line: no tree for {:?} (line {})",
                    self.language, line_idx
                );
                return spans;
            }
        };

        let _lang = match self.language.get_language() {
            Some(l) => l,
            None => {
                debug!("highlight_line: no lang for {:?}", self.language);
                return spans;
            }
        };

        let query = match &self.query {
            Some(q) => q,
            None => {
                return spans;
            }
        };

        // Bounds check: ensure byte ranges are valid for current content
        let content_len = content.len();
        if line_start_byte >= content_len {
            return spans;
        }

        let line_end_byte = (line_start_byte + line_text.len()).min(content_len);

        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(line_start_byte..line_end_byte);

        // Use catch_unwind to prevent panics from tree-sitter when tree is stale
        let captures_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut result = Vec::new();
            let mut captures = cursor.captures(&query, tree.root_node(), content.as_bytes());

            while let Some(&(ref m, capture_idx)) = captures.next() {
                let capture = m.captures[capture_idx];
                let node = capture.node;
                let start_byte = node.start_byte();
                let end_byte = node.end_byte();

                // Skip nodes outside our line or with invalid ranges
                if start_byte >= content_len || end_byte > content_len {
                    continue;
                }
                if start_byte >= line_end_byte || end_byte <= line_start_byte {
                    continue;
                }

                let start_col = start_byte.saturating_sub(line_start_byte);
                let end_col = (end_byte - line_start_byte).min(line_text.len());

                let capture_name = &query.capture_names()[capture.index as usize];
                let style = capture_to_style(capture_name);

                result.push(HighlightSpan {
                    start_col,
                    end_col,
                    style,
                });
            }
            result
        }));

        match captures_result {
            Ok(mut result) => {
                result.sort_by_key(|s| s.start_col);
                result
            }
            Err(_) => {
                // Tree was stale, return no highlights
                debug!("highlight_line: tree-sitter panic caught, tree is stale");
                spans
            }
        }
    }
}

fn capture_to_style(capture_name: &str) -> TextStyle {
    match capture_name {
        "keyword" | "keyword.control" | "keyword.function" | "keyword.operator"
        | "keyword.return" => TextStyle::Keyword,
        "type" | "type.builtin" | "constructor" => TextStyle::Type,
        "function" | "function.method" | "function.builtin" => TextStyle::Function,
        "string" | "string.special" => TextStyle::String,
        "number" | "float" => TextStyle::Number,
        "comment" | "comment.line" | "comment.block" => TextStyle::Comment,
        "operator" => TextStyle::Operator,
        "variable" | "variable.builtin" | "variable.parameter" => TextStyle::Variable,
        "constant" | "constant.builtin" => TextStyle::Constant,
        "attribute" | "label" => TextStyle::Attribute,
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" => TextStyle::Punctuation,
        "property" | "field" => TextStyle::Variable, // Map to Variable (no separate Property style)
        _ => TextStyle::Normal,
    }
}

const RUST_HIGHLIGHTS: &str = r#"
(line_comment) @comment
(block_comment) @comment

"fn" @keyword.function
"let" @keyword
"mut" @keyword
"const" @keyword
"static" @keyword
"if" @keyword.control
"else" @keyword.control
"match" @keyword.control
"for" @keyword.control
"while" @keyword.control
"loop" @keyword.control
"break" @keyword.control
"continue" @keyword.control
"return" @keyword.return
"pub" @keyword
"mod" @keyword
"use" @keyword
"struct" @keyword
"enum" @keyword
"impl" @keyword
"trait" @keyword
"type" @keyword
"where" @keyword
"as" @keyword
"in" @keyword
"ref" @keyword
"self" @keyword
"Self" @type.builtin
"async" @keyword
"await" @keyword
"move" @keyword
"unsafe" @keyword
"extern" @keyword
"crate" @keyword
"super" @keyword

(primitive_type) @type.builtin
(type_identifier) @type

(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))

(string_literal) @string
(raw_string_literal) @string
(char_literal) @string

(integer_literal) @number
(float_literal) @float

(boolean_literal) @constant.builtin

(identifier) @variable
(field_identifier) @property
(self) @variable.builtin

(attribute_item) @attribute
"#;

const PYTHON_HIGHLIGHTS: &str = r#"
(comment) @comment

"def" @keyword.function
"class" @keyword
"if" @keyword.control
"elif" @keyword.control
"else" @keyword.control
"for" @keyword.control
"while" @keyword.control
"try" @keyword.control
"except" @keyword.control
"finally" @keyword.control
"with" @keyword.control
"return" @keyword.return
"yield" @keyword.return
"import" @keyword
"from" @keyword
"as" @keyword
"pass" @keyword
"break" @keyword.control
"continue" @keyword.control
"raise" @keyword
"global" @keyword
"nonlocal" @keyword
"lambda" @keyword.function
"and" @keyword.operator
"or" @keyword.operator
"not" @keyword.operator
"in" @keyword.operator
"is" @keyword.operator
"async" @keyword
"await" @keyword

;; True / False / None
((identifier) @constant.builtin
  (#match? @constant.builtin "^(True|False|None)$"))

(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(call function: (attribute attribute: (identifier) @function.method))

(class_definition name: (identifier) @type)

(string) @string
(integer) @number
(float) @float

(identifier) @variable
(attribute attribute: (identifier) @property)
"#;

const JS_HIGHLIGHTS: &str = r#"
(comment) @comment

"function" @keyword.function
"const" @keyword
"let" @keyword
"var" @keyword
"if" @keyword.control
"else" @keyword.control
"for" @keyword.control
"while" @keyword.control
"do" @keyword.control
"switch" @keyword.control
"case" @keyword.control
"default" @keyword.control
"break" @keyword.control
"continue" @keyword.control
"return" @keyword.return
"throw" @keyword
"try" @keyword.control
"catch" @keyword.control
"finally" @keyword.control
"class" @keyword
"extends" @keyword
"new" @keyword
"this" @variable.builtin
"super" @variable.builtin
"import" @keyword
"export" @keyword
"from" @keyword
"async" @keyword
"await" @keyword
"typeof" @keyword.operator
"instanceof" @keyword.operator
"in" @keyword.operator

"true" @constant.builtin
"false" @constant.builtin
"null" @constant.builtin
"undefined" @constant.builtin

(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function.method))

(class_declaration name: (identifier) @type)

(string) @string
(template_string) @string
(number) @number

(identifier) @variable
(property_identifier) @property
"#;

const JSON_HIGHLIGHTS: &str = r#"
(string) @string
(number) @number
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin
(pair key: (string) @property)
"#;

const MD_HIGHLIGHTS: &str = r#"
(atx_heading) @keyword
(setext_heading) @keyword
(emphasis) @string
(strong_emphasis) @string
(link_destination) @function
(code_span) @string
(fenced_code_block) @string
"#;
