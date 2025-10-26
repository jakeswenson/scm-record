//! Tree-sitter based semantic analysis for source code.
//!
//! This module provides semantic-level parsing of source code changes using tree-sitter,
//! enabling users to select changes at semantic boundaries (functions, classes, methods)
//! rather than just line-by-line.

use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// Supported programming languages for semantic analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    /// Rust programming language
    Rust,
    /// Kotlin programming language
    Kotlin,
    /// Java programming language
    Java,
    /// HCL (HashiCorp Configuration Language) / Terraform
    Hcl,
    /// Python programming language
    Python,
    /// Nushell scripting language
    Nushell,
}

impl SourceLanguage {
    /// Detect language from file path/extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension {
            "rs" => Some(Self::Rust),
            "kt" | "kts" => Some(Self::Kotlin),
            "java" => Some(Self::Java),
            "tf" | "hcl" => Some(Self::Hcl),
            "py" | "pyw" => Some(Self::Python),
            "nu" => Some(Self::Nushell),
            _ => None,
        }
    }

    /// Get the tree-sitter Language for this source language.
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Nushell => tree_sitter_nu::LANGUAGE.into(),
        }
    }

    /// Get the tree-sitter query for extracting semantic nodes.
    ///
    /// These queries identify important semantic constructs like functions,
    /// classes, methods, etc. for each language.
    pub fn semantic_query(&self) -> &'static str {
        match self {
            Self::Rust => {
                r#"
                (function_item
                  name: (identifier) @name) @function

                (struct_item
                  name: (type_identifier) @name) @struct

                (enum_item
                  name: (type_identifier) @name) @enum

                (impl_item
                  type: (type_identifier) @name) @impl

                (trait_item
                  name: (type_identifier) @name) @trait

                (mod_item
                  name: (identifier) @name) @module
                "#
            }
            Self::Kotlin => {
                r#"
                (function_declaration
                  (simple_identifier) @name) @function

                (class_declaration
                  (type_identifier) @name) @class

                (object_declaration
                  (type_identifier) @name) @object
                "#
            }
            Self::Java => {
                r#"
                (method_declaration
                  name: (identifier) @name) @method

                (class_declaration
                  name: (identifier) @name) @class

                (interface_declaration
                  name: (identifier) @name) @interface

                (enum_declaration
                  name: (identifier) @name) @enum
                "#
            }
            Self::Hcl => {
                r#"
                (block
                  (identifier) @type
                  (string_lit)? @name) @block
                "#
            }
            Self::Python => {
                r#"
                (function_definition
                  name: (identifier) @name) @function

                (class_definition
                  name: (identifier) @name) @class
                "#
            }
            Self::Nushell => {
                r#"
                (decl_def
                  name: (cmd_identifier) @name) @function
                "#
            }
        }
    }
}

/// Type of semantic node (function, class, method, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticNodeType {
    /// Function or method
    Function,
    /// Class or struct
    Class,
    /// Module or namespace
    Module,
    /// Impl block (Rust)
    Impl,
    /// Trait definition (Rust)
    Trait,
    /// Enum definition
    Enum,
    /// Interface (Java)
    Interface,
    /// Object (Kotlin)
    Object,
    /// Block (HCL)
    Block,
    /// Other/unknown semantic node
    Other,
}

/// A semantic node representing a code construct (function, class, etc.)
#[derive(Debug, Clone)]
pub struct SemanticNode {
    /// Type of this node
    pub node_type: SemanticNodeType,
    /// Name of the construct (if available)
    pub name: Option<String>,
    /// Start line (0-indexed)
    pub start_line: usize,
    /// End line (0-indexed, inclusive)
    pub end_line: usize,
    /// Child nodes (for nested structures)
    pub children: Vec<SemanticNode>,
}

/// Parse source code and extract semantic nodes.
pub fn parse_semantic_nodes(
    language: SourceLanguage,
    source_code: &str,
) -> Result<Vec<SemanticNode>, String> {
    let mut parser = Parser::new();
    let ts_language = language.tree_sitter_language();
    parser
        .set_language(&ts_language)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let tree = parser
        .parse(source_code, None)
        .ok_or_else(|| "Failed to parse source code".to_string())?;

    extract_nodes(language, &tree, source_code)
}

fn extract_nodes(
    language: SourceLanguage,
    tree: &Tree,
    source_code: &str,
) -> Result<Vec<SemanticNode>, String> {
    let query_str = language.semantic_query();
    let ts_language = language.tree_sitter_language();
    let query = Query::new(&ts_language, query_str)
        .map_err(|e| format!("Failed to create query: {}", e))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());

    let mut nodes = Vec::new();

    for match_ in matches {
        let mut node_type = SemanticNodeType::Other;
        let mut name = None;
        let mut start_line = 0;
        let mut end_line = 0;

        for capture in match_.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            let captured_node = capture.node;

            match capture_name.as_str() {
                "function" => {
                    node_type = SemanticNodeType::Function;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "method" => {
                    node_type = SemanticNodeType::Function;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "class" | "struct" => {
                    node_type = SemanticNodeType::Class;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "module" => {
                    node_type = SemanticNodeType::Module;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "impl" => {
                    node_type = SemanticNodeType::Impl;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "trait" => {
                    node_type = SemanticNodeType::Trait;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "enum" => {
                    node_type = SemanticNodeType::Enum;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "interface" => {
                    node_type = SemanticNodeType::Interface;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "object" => {
                    node_type = SemanticNodeType::Object;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "block" => {
                    node_type = SemanticNodeType::Block;
                    start_line = captured_node.start_position().row;
                    end_line = captured_node.end_position().row;
                }
                "name" => {
                    let text = captured_node
                        .utf8_text(source_code.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    name = text;
                }
                "type" => {
                    // For HCL blocks, the type is the block identifier
                    let text = captured_node
                        .utf8_text(source_code.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                    if name.is_none() {
                        name = text;
                    }
                }
                _ => {}
            }
        }

        nodes.push(SemanticNode {
            node_type,
            name,
            start_line,
            end_line,
            children: Vec::new(),
        });
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            SourceLanguage::from_path(Path::new("foo.rs")),
            Some(SourceLanguage::Rust)
        );
        assert_eq!(
            SourceLanguage::from_path(Path::new("foo.kt")),
            Some(SourceLanguage::Kotlin)
        );
        assert_eq!(
            SourceLanguage::from_path(Path::new("foo.java")),
            Some(SourceLanguage::Java)
        );
        assert_eq!(
            SourceLanguage::from_path(Path::new("main.tf")),
            Some(SourceLanguage::Hcl)
        );
        assert_eq!(
            SourceLanguage::from_path(Path::new("script.py")),
            Some(SourceLanguage::Python)
        );
        assert_eq!(
            SourceLanguage::from_path(Path::new("script.nu")),
            Some(SourceLanguage::Nushell)
        );
        assert_eq!(SourceLanguage::from_path(Path::new("unknown.txt")), None);
    }

    #[test]
    fn test_rust_parsing() {
        let source = r#"
fn main() {
    println!("Hello, world!");
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}
"#;

        let nodes = parse_semantic_nodes(SourceLanguage::Rust, source).unwrap();
        assert!(nodes.len() >= 3); // main, Point struct, Point impl

        // Check that we found the main function
        assert!(nodes.iter().any(|n| {
            n.node_type == SemanticNodeType::Function && n.name.as_deref() == Some("main")
        }));

        // Check that we found the Point struct
        assert!(nodes.iter().any(|n| {
            n.node_type == SemanticNodeType::Class && n.name.as_deref() == Some("Point")
        }));
    }

    #[test]
    fn test_python_parsing() {
        let source = r#"
def hello():
    print("Hello, world!")

class Person:
    def __init__(self, name):
        self.name = name
"#;

        let nodes = parse_semantic_nodes(SourceLanguage::Python, source).unwrap();
        assert!(nodes.len() >= 2); // hello function and Person class

        assert!(nodes.iter().any(|n| {
            n.node_type == SemanticNodeType::Function && n.name.as_deref() == Some("hello")
        }));

        assert!(nodes.iter().any(|n| {
            n.node_type == SemanticNodeType::Class && n.name.as_deref() == Some("Person")
        }));
    }

    #[test]
    fn test_java_parsing() {
        let source = r#"
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

        let nodes = parse_semantic_nodes(SourceLanguage::Java, source).unwrap();
        assert!(nodes.len() >= 1); // HelloWorld class

        assert!(nodes.iter().any(|n| {
            n.node_type == SemanticNodeType::Class && n.name.as_deref() == Some("HelloWorld")
        }));
    }
}
