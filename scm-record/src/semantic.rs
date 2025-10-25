//! Semantic analysis using tree-sitter for syntax-aware change selection.
//!
//! This module provides tree-sitter integration via tree-house to enable selecting changes at
//! semantic boundaries (functions, classes, methods, etc.) rather than just at
//! the line level.

#![cfg(feature = "tree-sitter")]

use std::path::Path;
use tree_house::{Language as TreeHouseLanguage, Parser, Query, QueryCursor};

/// Represents the type of a semantic node in the source code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticNodeType {
    /// A function or method definition
    Function,
    /// A struct, class, or type definition
    Struct,
    /// An impl block
    Impl,
    /// A module
    Module,
    /// A code block (if, for, while, etc.)
    Block,
    /// Other semantic constructs
    Other(String),
}

/// Metadata about a semantic node in the source code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticNode {
    /// The type of this semantic node
    pub node_type: SemanticNodeType,
    /// The name of this node (e.g., function name), if available
    pub name: Option<String>,
    /// The starting line (0-indexed)
    pub start_line: usize,
    /// The ending line (0-indexed, inclusive)
    pub end_line: usize,
    /// Child semantic nodes
    pub children: Vec<SemanticNode>,
}

/// Language detection based on file extension and content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    /// Rust programming language
    Rust,
    /// Python programming language
    Python,
    /// TypeScript
    TypeScript,
    /// JavaScript
    JavaScript,
    /// Unknown or unsupported language
    Unknown,
}

impl Language {
    /// Detect the language from a file path.
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") => Language::JavaScript,
            _ => Language::Unknown,
        }
    }

    /// Check if tree-sitter support is available for this language.
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Language::Rust | Language::Python | Language::TypeScript | Language::JavaScript
        )
    }
}

/// Parse source code and extract semantic nodes.
///
/// This function takes source code content and returns a tree of semantic nodes
/// that can be used for syntax-aware selection.
pub fn parse_semantic_nodes(language: Language, source: &str) -> Option<Vec<SemanticNode>> {
    match language {
        Language::Rust => parse_rust(source),
        _ => None, // Other languages not yet implemented
    }
}

/// Parse Rust source code using tree-house.
fn parse_rust(source: &str) -> Option<Vec<SemanticNode>> {
    let mut parser = Parser::new();
    let rust_lang = TreeHouseLanguage::rust();
    parser.set_language(rust_lang).ok()?;

    let tree = parser.parse(source.as_bytes(), None)?;
    let root_node = tree.root_node();

    // Query for interesting Rust constructs
    let query_source = r#"
        (function_item
            name: (identifier) @fn.name) @fn.def

        (struct_item
            name: (type_identifier) @struct.name) @struct.def

        (impl_item
            type: (type_identifier) @impl.type) @impl.def

        (mod_item
            name: (identifier) @mod.name) @mod.def
    "#;

    let query = Query::new(rust_lang, query_source).ok()?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, root_node, source.as_bytes());

    let mut nodes = Vec::new();

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let start_line = node.start_position().row;
            let end_line = node.end_position().row;

            // Determine node type based on capture name
            let capture_name = query.capture_names()[capture.index as usize];

            let (node_type, name) = if capture_name.ends_with(".def") {
                // This is a definition node, find its name from other captures
                let name_text = if capture_name.starts_with("fn") {
                    Some(get_text_for_node(source, node, "fn.name", &match_, &query))
                } else if capture_name.starts_with("struct") {
                    Some(get_text_for_node(source, node, "struct.name", &match_, &query))
                } else if capture_name.starts_with("impl") {
                    Some(get_text_for_node(source, node, "impl.type", &match_, &query))
                } else if capture_name.starts_with("mod") {
                    Some(get_text_for_node(source, node, "mod.name", &match_, &query))
                } else {
                    None
                };

                let node_type = if capture_name.starts_with("fn") {
                    SemanticNodeType::Function
                } else if capture_name.starts_with("struct") {
                    SemanticNodeType::Struct
                } else if capture_name.starts_with("impl") {
                    SemanticNodeType::Impl
                } else if capture_name.starts_with("mod") {
                    SemanticNodeType::Module
                } else {
                    SemanticNodeType::Other(capture_name.to_string())
                };

                (node_type, name_text)
            } else {
                continue; // Skip name captures, we only want definitions
            };

            nodes.push(SemanticNode {
                node_type,
                name,
                start_line,
                end_line,
                children: Vec::new(), // TODO: Parse nested structures
            });
        }
    }

    Some(nodes)
}

/// Helper function to extract text for a named capture.
fn get_text_for_node(
    source: &str,
    _parent: tree_house::Node,
    capture_name: &str,
    match_: &tree_house::QueryMatch,
    query: &Query,
) -> Option<String> {
    for capture in match_.captures {
        if query.capture_names()[capture.index as usize] == capture_name {
            let text = &source[capture.node.byte_range()];
            return Some(text.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_path(&PathBuf::from("foo.rs")), Language::Rust);
        assert_eq!(
            Language::from_path(&PathBuf::from("bar.py")),
            Language::Python
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("baz.ts")),
            Language::TypeScript
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("qux.js")),
            Language::JavaScript
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("unknown.txt")),
            Language::Unknown
        );
    }

    #[test]
    fn test_language_support() {
        assert!(Language::Rust.is_supported());
        assert!(Language::Python.is_supported());
        assert!(Language::TypeScript.is_supported());
        assert!(Language::JavaScript.is_supported());
        assert!(!Language::Unknown.is_supported());
    }

    #[test]
    fn test_parse_rust_functions() {
        let source = r#"
fn hello_world() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

        let nodes = parse_semantic_nodes(Language::Rust, source);
        assert!(nodes.is_some());

        let nodes = nodes.unwrap();
        assert_eq!(nodes.len(), 2);

        // Check first function
        assert_eq!(nodes[0].node_type, SemanticNodeType::Function);
        assert_eq!(nodes[0].name, Some("hello_world".to_string()));

        // Check second function
        assert_eq!(nodes[1].node_type, SemanticNodeType::Function);
        assert_eq!(nodes[1].name, Some("add".to_string()));
    }

    #[test]
    fn test_parse_rust_structs() {
        let source = r#"
struct Point {
    x: f64,
    y: f64,
}

struct Person {
    name: String,
    age: u32,
}
"#;

        let nodes = parse_semantic_nodes(Language::Rust, source);
        assert!(nodes.is_some());

        let nodes = nodes.unwrap();
        assert_eq!(nodes.len(), 2);

        assert_eq!(nodes[0].node_type, SemanticNodeType::Struct);
        assert_eq!(nodes[0].name, Some("Point".to_string()));

        assert_eq!(nodes[1].node_type, SemanticNodeType::Struct);
        assert_eq!(nodes[1].name, Some("Person".to_string()));
    }

    #[test]
    fn test_parse_unsupported_language() {
        let source = "some text content";
        let nodes = parse_semantic_nodes(Language::Unknown, source);
        assert!(nodes.is_none());
    }
}
