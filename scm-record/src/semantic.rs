//! Semantic analysis using tree-sitter for syntax-aware change selection.
//!
//! This module provides tree-sitter integration to enable selecting changes at
//! semantic boundaries (functions, classes, methods, etc.) rather than just at
//! the line level.

#![cfg(feature = "tree-sitter")]

use std::path::Path;
use tree_sitter::{Language as TSLanguage, Parser, Query, QueryCursor};

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
    /// Kotlin programming language
    Kotlin,
    /// Java programming language
    Java,
    /// HCL (HashiCorp Configuration Language)
    Hcl,
    /// Python programming language
    Python,
    /// Unknown or unsupported language
    Unknown,
}

impl Language {
    /// Detect the language from a file path.
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("kt") | Some("kts") => Language::Kotlin,
            Some("java") => Language::Java,
            Some("hcl") | Some("tf") | Some("tfvars") => Language::Hcl,
            Some("py") | Some("pyw") => Language::Python,
            _ => Language::Unknown,
        }
    }

    /// Check if tree-sitter support is available for this language.
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Language::Rust | Language::Kotlin | Language::Java | Language::Hcl | Language::Python
        )
    }

    /// Get the tree-sitter Language for this language.
    fn tree_sitter_language(&self) -> Option<TSLanguage> {
        match self {
            Language::Rust => Some(unsafe { tree_sitter_rust::LANGUAGE.into() }),
            Language::Kotlin => {
                // tree-sitter-kotlin may use a different version of tree-sitter
                // We convert it by transmuting the underlying pointer
                let lang = tree_sitter_kotlin::language();
                Some(unsafe {
                    std::mem::transmute(lang)
                })
            }
            Language::Java => Some(unsafe { tree_sitter_java::LANGUAGE.into() }),
            Language::Hcl => Some(tree_sitter_hcl::LANGUAGE.into()),
            Language::Python => Some(unsafe { tree_sitter_python::LANGUAGE.into() }),
            Language::Unknown => None,
        }
    }
}

/// Parse source code and extract semantic nodes.
///
/// This function takes source code content and returns a tree of semantic nodes
/// that can be used for syntax-aware selection.
pub fn parse_semantic_nodes(language: Language, source: &str) -> Option<Vec<SemanticNode>> {
    let ts_language = language.tree_sitter_language()?;
    let query_source = get_query_for_language(language)?;

    let mut parser = Parser::new();
    parser.set_language(&ts_language).ok()?;

    let tree = parser.parse(source, None)?;
    let root_node = tree.root_node();

    let query = Query::new(&ts_language, query_source).ok()?;
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, root_node, source.as_bytes());

    let mut nodes = Vec::new();
    let mut seen_nodes = std::collections::HashSet::new();

    // Manually iterate using the streaming iterator pattern
    loop {
        // Try to get the next capture
        let capture_data = {
            // This scope ensures we don't hold onto any references
            use streaming_iterator::StreamingIterator;

            match captures.next() {
                Some((qmatch, capture_idx)) => {
                    // capture_idx is a reference to usize, dereference it
                    let idx = *capture_idx;
                    // Get the capture from the slice
                    if let Some(capture) = qmatch.captures.get(idx) {
                        let node_id = capture.node.id();
                        let node_range = capture.node.range();
                        let capture_index = capture.index;

                        Some(Some((node_id, node_range, capture_index)))
                    } else {
                        Some(None)
                    }
                }
                None => None,
            }
        };

        match capture_data {
            Some(Some((node_id, node_range, capture_index))) => {
                let capture_name = query.capture_names()[capture_index as usize];

                // Only process definition captures, and only once per node
                if capture_name.ends_with(".def") && !seen_nodes.contains(&node_id) {
                    seen_nodes.insert(node_id);

                    let start_line = node_range.start_point.row;
                    let end_line = node_range.end_point.row;

                    // Extract name from the source text
                    let name_text =
                        extract_name_from_range(source, node_range.start_byte, node_range.end_byte);
                    let node_type = parse_node_type(&capture_name);

                    nodes.push(SemanticNode {
                        node_type,
                        name: name_text,
                        start_line,
                        end_line,
                        children: Vec::new(), // TODO: Parse nested structures
                    });
                }
            }
            Some(None) => continue, // Skip invalid captures
            None => break,
        }
    }

    Some(nodes)
}

/// Get the appropriate tree-sitter query for a language.
fn get_query_for_language(language: Language) -> Option<&'static str> {
    match language {
        Language::Rust => Some(RUST_QUERY),
        Language::Kotlin => Some(KOTLIN_QUERY),
        Language::Java => Some(JAVA_QUERY),
        Language::Hcl => Some(HCL_QUERY),
        Language::Python => Some(PYTHON_QUERY),
        Language::Unknown => None,
    }
}

/// Parse node type from capture name.
fn parse_node_type(capture_name: &str) -> SemanticNodeType {
    if capture_name.starts_with("fn") || capture_name.starts_with("function") {
        SemanticNodeType::Function
    } else if capture_name.starts_with("struct") || capture_name.starts_with("class") {
        SemanticNodeType::Struct
    } else if capture_name.starts_with("impl") {
        SemanticNodeType::Impl
    } else if capture_name.starts_with("mod") || capture_name.starts_with("module") {
        SemanticNodeType::Module
    } else {
        SemanticNodeType::Other(capture_name.to_string())
    }
}

/// Helper function to extract a name from a byte range in the source.
/// This is a simple heuristic that looks for identifier-like tokens.
fn extract_name_from_range(source: &str, start_byte: usize, end_byte: usize) -> Option<String> {
    let text = &source[start_byte..end_byte];

    // Try to find the first identifier-like word (letters, numbers, underscores)
    for word in text.split_whitespace() {
        // Skip keywords and common syntax elements
        if matches!(
            word,
            "fn" | "function"
                | "struct"
                | "class"
                | "impl"
                | "mod"
                | "module"
                | "def"
                | "pub"
                | "private"
                | "public"
                | "{"
                | "}"
                | "("
                | ")"
        ) {
            continue;
        }

        // Look for identifier-like patterns
        if word.chars().all(|c| c.is_alphanumeric() || c == '_') && !word.is_empty() {
            return Some(word.to_string());
        }

        // Handle cases like "Point {" or "new()"
        if let Some(name) = word.split(|c: char| !c.is_alphanumeric() && c != '_').next() {
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Some(name.to_string());
            }
        }
    }

    None
}

// Tree-sitter query constants for each language

const RUST_QUERY: &str = r#"
(function_item
    name: (identifier) @fn.name) @fn.def

(struct_item
    name: (type_identifier) @struct.name) @struct.def

(impl_item
    type: (type_identifier) @impl.type) @impl.def

(mod_item
    name: (identifier) @mod.name) @mod.def
"#;

const KOTLIN_QUERY: &str = r#"
(function_declaration
    (simple_identifier) @function.name) @function.def

(class_declaration
    (type_identifier) @class.name) @class.def
"#;

const JAVA_QUERY: &str = r#"
(method_declaration
    name: (identifier) @function.name) @function.def

(class_declaration
    name: (identifier) @class.name) @class.def
"#;

const HCL_QUERY: &str = r#"
(block
    (identifier) @module.name) @module.def
"#;

const PYTHON_QUERY: &str = r#"
(function_definition
    name: (identifier) @function.name) @function.def

(class_definition
    name: (identifier) @class.name) @class.def
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            Language::from_path(&PathBuf::from("foo.rs")),
            Language::Rust
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("bar.py")),
            Language::Python
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("baz.kt")),
            Language::Kotlin
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("qux.java")),
            Language::Java
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("main.tf")),
            Language::Hcl
        );
        assert_eq!(
            Language::from_path(&PathBuf::from("unknown.txt")),
            Language::Unknown
        );
    }

    #[test]
    fn test_language_support() {
        assert!(Language::Rust.is_supported());
        assert!(Language::Kotlin.is_supported());
        assert!(Language::Java.is_supported());
        assert!(Language::Hcl.is_supported());
        assert!(Language::Python.is_supported());
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
