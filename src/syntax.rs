use log::debug;
use ratatui::style::{Color, Modifier, Style};
use std::path::Path;
use tree_sitter::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    language: SyntaxLanguage,
    query: Option<Query>,
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
    pub style: Style,
}

impl SyntaxHighlighter {
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
        }
    }

    pub fn parse(&mut self, content: &str) {
        if self.language == SyntaxLanguage::Plain {
            self.tree = None;
            return;
        }

        // Do a fresh parse each time.
        // Note: Tree-sitter supports incremental parsing by passing the old tree,
        // but that requires calling tree.edit() with edit info before re-parsing.
        // For now, we do a full reparse which is simpler and correct.
        let new_tree = self.parser.parse(content, None);
        if new_tree.is_none() {
            debug!("parse returned None for {:?}", self.language);
        } else {
            debug!("parse OK for {:?}", self.language);
        }
        self.tree = new_tree;
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

fn capture_to_style(capture_name: &str) -> Style {
    match capture_name {
        "keyword" | "keyword.control" | "keyword.function" | "keyword.operator"
        | "keyword.return" => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        "type" | "type.builtin" | "constructor" => Style::default().fg(Color::Yellow),
        "function" | "function.method" | "function.builtin" => Style::default().fg(Color::Blue),
        "string" | "string.special" => Style::default().fg(Color::Green),
        "number" | "float" => Style::default().fg(Color::Cyan),
        "comment" | "comment.line" | "comment.block" => Style::default().fg(Color::DarkGray),
        "operator" => Style::default().fg(Color::Red),
        "variable" | "variable.builtin" | "variable.parameter" => Style::default().fg(Color::White),
        "constant" | "constant.builtin" => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        "attribute" | "label" => Style::default().fg(Color::Yellow),
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" => {
            Style::default().fg(Color::White)
        }
        "property" | "field" => Style::default().fg(Color::LightBlue),
        _ => Style::default(),
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
