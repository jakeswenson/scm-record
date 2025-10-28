//! Semantic navigation using tree-sitter for code structure parsing.
//!
//! This module provides semantic-first navigation where changes are organized
//! by code structure (containers/members) rather than diff proximity.

use std::path::Path;

#[cfg(feature = "tree-sitter")]
use tree_sitter::{Language, Parser, Tree};

/// Supported languages for semantic parsing in First Wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLanguage {
    /// Rust programming language (.rs)
    Rust,
    /// Kotlin programming language (.kt, .kts)
    Kotlin,
    /// Java programming language (.java)
    Java,
    /// HCL (Terraform/OpenTofu) configuration (.tf, .hcl)
    Hcl,
    /// Python programming language (.py)
    Python,
    /// Markdown documentation (.md)
    Markdown,
    /// YAML configuration (.yaml, .yml)
    Yaml,
}

impl SupportedLanguage {
    /// Detect language from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension {
            "rs" => Some(Self::Rust),
            "kt" | "kts" => Some(Self::Kotlin),
            "java" => Some(Self::Java),
            "tf" | "hcl" => Some(Self::Hcl),
            "py" => Some(Self::Python),
            "md" => Some(Self::Markdown),
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }

    /// Get the human-readable name of the language.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Kotlin => "Kotlin",
            Self::Java => "Java",
            Self::Hcl => "HCL",
            Self::Python => "Python",
            Self::Markdown => "Markdown",
            Self::Yaml => "YAML",
        }
    }

    /// Get the tree-sitter language grammar for this language.
    #[cfg(feature = "tree-sitter")]
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Markdown => tree_sitter_md::LANGUAGE.into(),
            Self::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        }
    }
}

/// Creates and configures a tree-sitter parser for the given language.
#[cfg(feature = "tree-sitter")]
pub fn create_parser(language: SupportedLanguage) -> Result<Parser, SemanticError> {
    let mut parser = Parser::new();
    let ts_language = language.tree_sitter_language();
    parser
        .set_language(&ts_language)
        .map_err(|e| SemanticError::ParserSetup {
            language: language.name(),
            error: e.to_string(),
        })?;
    Ok(parser)
}

/// Parse source code into a tree-sitter syntax tree.
#[cfg(feature = "tree-sitter")]
pub fn parse_source(parser: &mut Parser, source: &str) -> Result<Tree, SemanticError> {
    parser
        .parse(source, None)
        .ok_or(SemanticError::ParseFailed)
}

/// Parsed version of a file with its tree-sitter syntax tree.
#[cfg(feature = "tree-sitter")]
pub struct ParsedFile {
    /// The source code
    pub source: String,
    /// The parsed syntax tree
    pub tree: Tree,
}

/// Parse both old and new versions of a file.
#[cfg(feature = "tree-sitter")]
pub fn parse_file_versions(
    language: SupportedLanguage,
    old_source: &str,
    new_source: &str,
) -> Result<(ParsedFile, ParsedFile), SemanticError> {
    let mut parser = create_parser(language)?;

    let old_tree = parse_source(&mut parser, old_source)?;
    let new_tree = parse_source(&mut parser, new_source)?;

    Ok((
        ParsedFile {
            source: old_source.to_string(),
            tree: old_tree,
        },
        ParsedFile {
            source: new_source.to_string(),
            tree: new_tree,
        },
    ))
}

/// Information about a semantic container (struct, class, impl, function, etc.) extracted from the AST.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    /// The type of container
    pub kind: ContainerKind,
    /// The name of the container
    pub name: String,
    /// Start line number (0-indexed)
    pub start_line: usize,
    /// End line number (0-indexed)
    pub end_line: usize,
}

/// The kind of semantic container, generalized across languages.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerKind {
    /// A struct definition (Rust)
    Struct,
    /// A class definition (Kotlin, Java, Python)
    Class,
    /// An interface definition (Kotlin, Java)
    Interface,
    /// An enum definition (Kotlin, Java)
    Enum,
    /// An object declaration (Kotlin)
    Object,
    /// An impl block (Rust)
    Impl {
        /// The trait being implemented, if any
        trait_name: Option<String>,
    },
    /// A top-level function
    Function,
    /// An HCL resource block
    Resource {
        /// Resource type (e.g., "aws_instance")
        resource_type: String,
    },
    /// An HCL data source block
    DataSource {
        /// Data source type (e.g., "aws_ami")
        data_type: String,
    },
    /// An HCL variable declaration
    Variable,
    /// An HCL output declaration
    Output,
    /// An HCL module block
    Module,
    /// A Markdown section (header)
    Section {
        /// Header level (1-6)
        level: usize,
    },
}

/// Extract Rust containers from a parsed syntax tree.
#[cfg(feature = "tree-sitter")]
pub fn extract_rust_containers(parsed: &ParsedFile) -> Vec<Container> {
    let mut containers = Vec::new();
    let root_node = parsed.tree.root_node();
    let source_bytes = parsed.source.as_bytes();

    // Walk through top-level items in the source file
    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        match child.kind() {
            "struct_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    containers.push(Container {
                        kind: ContainerKind::Struct,
                        name,
                        start_line: child.start_position().row,
                        end_line: child.end_position().row,
                    });
                }
            }
            "impl_item" => {
                // Extract type name and optional trait name
                let type_node = child.child_by_field_name("type");
                let trait_node = child.child_by_field_name("trait");

                if let Some(type_node) = type_node {
                    let type_name = type_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let trait_name = trait_node.and_then(|node| {
                        node.utf8_text(source_bytes).ok().map(|s| s.to_string())
                    });

                    containers.push(Container {
                        kind: ContainerKind::Impl { trait_name },
                        name: type_name,
                        start_line: child.start_position().row,
                        end_line: child.end_position().row,
                    });
                }
            }
            "function_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    containers.push(Container {
                        kind: ContainerKind::Function,
                        name,
                        start_line: child.start_position().row,
                        end_line: child.end_position().row,
                    });
                }
            }
            _ => {}
        }
    }

    containers
}

/// Information about a semantic member (field, method, property) extracted from the AST.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The type of member
    pub kind: MemberKind,
    /// The name of the member
    pub name: String,
    /// Start line number (0-indexed)
    pub start_line: usize,
    /// End line number (0-indexed)
    pub end_line: usize,
}

/// The kind of semantic member, generalized across languages.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberKind {
    /// A field (Rust, Kotlin, Java, Python)
    Field,
    /// A method (all languages)
    Method,
    /// A property (Kotlin, Python)
    Property,
}

/// Extract struct fields from a struct definition node.
#[cfg(feature = "tree-sitter")]
pub fn extract_struct_fields(
    struct_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut fields = Vec::new();

    // Find the field_declaration_list
    if let Some(field_list) = struct_node.child_by_field_name("body") {
        let mut cursor = field_list.walk();
        for field in field_list.children(&mut cursor) {
            if field.kind() == "field_declaration" {
                if let Some(name_node) = field.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) = expand_range_for_attributes_and_comments(field, field_list);

                    fields.push(Member {
                        kind: MemberKind::Field,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
        }
    }

    fields
}

/// Extract methods from an impl block node.
#[cfg(feature = "tree-sitter")]
pub fn extract_impl_methods(
    impl_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut methods = Vec::new();

    // Find the declaration_list
    if let Some(decl_list) = impl_node.child_by_field_name("body") {
        let mut cursor = decl_list.walk();
        for item in decl_list.children(&mut cursor) {
            if item.kind() == "function_item" {
                if let Some(name_node) = item.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) = expand_range_for_attributes_and_comments(item, decl_list);

                    methods.push(Member {
                        kind: MemberKind::Method,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
        }
    }

    methods
}

/// Configuration for what trivia types to include when expanding node ranges.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone)]
struct TriviaConfig {
    /// Node kinds that should always be included (e.g., attributes, annotations, decorators)
    always_include: &'static [&'static str],
    /// Node kinds that should be included if adjacent (e.g., comments)
    adjacent_only: &'static [&'static str],
}

#[cfg(feature = "tree-sitter")]
impl TriviaConfig {
    /// Rust trivia configuration
    fn rust() -> Self {
        Self {
            always_include: &["attribute_item"], // #[test], #[cfg(...)]
            adjacent_only: &["line_comment", "block_comment"], // ///, /* */
        }
    }

    /// Kotlin trivia configuration
    fn kotlin() -> Self {
        Self {
            always_include: &["annotation"], // @Test, @JvmStatic
            adjacent_only: &["comment"], // //, /* */
        }
    }

    /// Java trivia configuration
    fn java() -> Self {
        Self {
            always_include: &["marker_annotation", "annotation"], // @Override, @Test
            adjacent_only: &["line_comment", "block_comment", "javadoc_comment"], // //, /* */, /** */
        }
    }

    /// Python trivia configuration
    fn python() -> Self {
        Self {
            always_include: &["decorator"], // @property, @staticmethod
            adjacent_only: &["comment"], // #
        }
    }

    /// HCL trivia configuration
    fn hcl() -> Self {
        Self {
            always_include: &[], // HCL doesn't have attributes/annotations
            adjacent_only: &["comment"], // #, //
        }
    }

    /// Generic fallback for languages without special trivia
    fn generic() -> Self {
        Self {
            always_include: &[],
            adjacent_only: &["comment"],
        }
    }
}

/// Expands a node's line range to include preceding trivia (attributes, comments, etc.).
///
/// This ensures that when we group sections by semantic structure, we include the full
/// declaration including doc comments, attributes/annotations/decorators, and surrounding whitespace.
///
/// The `config` parameter determines which node types are considered trivia for the language.
#[cfg(feature = "tree-sitter")]
fn expand_range_for_trivia(
    node: tree_sitter::Node,
    parent: tree_sitter::Node,
    config: &TriviaConfig,
) -> (usize, usize) {
    let mut start_line = node.start_position().row;
    let end_line = node.end_position().row;

    // Walk backwards through siblings to find trivia
    let mut cursor = parent.walk();
    let siblings: Vec<_> = parent.children(&mut cursor).collect();

    if let Some(node_index) = siblings.iter().position(|n| n.id() == node.id()) {
        // Look at all previous siblings in reverse order
        for sibling in siblings[..node_index].iter().rev() {
            let kind = sibling.kind();

            // Check if this is a trivia node that should always be included
            if config.always_include.contains(&kind) {
                start_line = start_line.min(sibling.start_position().row);
            }
            // Check if this is a trivia node that should only be included if adjacent
            else if config.adjacent_only.contains(&kind) {
                let sibling_line = sibling.start_position().row;
                // Only include if it's adjacent or within 1 line
                if start_line.saturating_sub(sibling_line) <= 1 {
                    start_line = sibling_line;
                } else {
                    break; // Stop if there's a gap
                }
            }
            // Stop at non-trivia siblings
            else if !kind.is_empty() {
                break;
            }
        }
    }

    (start_line, end_line)
}

/// Expands a node's line range to include preceding attributes and comments (Rust-specific wrapper).
///
/// This is a convenience wrapper around `expand_range_for_trivia` for Rust code.
#[cfg(feature = "tree-sitter")]
fn expand_range_for_attributes_and_comments(
    node: tree_sitter::Node,
    parent: tree_sitter::Node,
) -> (usize, usize) {
    expand_range_for_trivia(node, parent, &TriviaConfig::rust())
}

/// Extract methods from a Python class definition node.
#[cfg(feature = "tree-sitter")]
pub fn extract_python_methods(
    class_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut methods = Vec::new();

    // Find the class body (block node)
    if let Some(body) = class_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for item in body.children(&mut cursor) {
            // Python has function_definition nodes for methods
            // Can also have decorated_definition wrapping a function_definition
            let function_node = match item.kind() {
                "function_definition" => Some(item),
                "decorated_definition" => {
                    // Look for function_definition child
                    item.child_by_field_name("definition")
                        .filter(|n| n.kind() == "function_definition")
                }
                _ => None,
            };

            if let Some(func_node) = function_node {
                if let Some(name_node) = func_node.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    // Use the outer node (decorated_definition if present, otherwise function_definition)
                    // for proper range calculation
                    let range_node = if item.kind() == "decorated_definition" {
                        item
                    } else {
                        func_node
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(range_node, body, &TriviaConfig::python());

                    methods.push(Member {
                        kind: MemberKind::Method,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
        }
    }

    methods
}

/// Extract containers with their members from a parsed Python file.
#[cfg(feature = "tree-sitter")]
pub fn extract_python_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
    let mut containers = Vec::new();
    let root_node = parsed.tree.root_node();
    let source_bytes = parsed.source.as_bytes();

    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        // Check for class_definition or decorated_definition wrapping a class
        let (class_node, outer_node) = match child.kind() {
            "class_definition" => (Some(child), child),
            "decorated_definition" => {
                let def = child.child_by_field_name("definition");
                if let Some(class_def) = def.filter(|n| n.kind() == "class_definition") {
                    (Some(class_def), child)
                } else {
                    (None, child)
                }
            }
            _ => (None, child),
        };

        if let Some(class_def) = class_node {
            if let Some(name_node) = class_def.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(source_bytes)
                    .unwrap_or("<unknown>")
                    .to_string();

                let methods = extract_python_methods(class_def, source_bytes);
                let (start_line, end_line) =
                    expand_range_for_trivia(outer_node, root_node, &TriviaConfig::python());

                containers.push(ContainerWithMembers {
                    container: Container {
                        kind: ContainerKind::Class,
                        name,
                        start_line,
                        end_line,
                    },
                    members: methods,
                });
            }
        }
        // Check for top-level function_definition
        else if child.kind() == "function_definition"
            || (child.kind() == "decorated_definition"
                && child
                    .child_by_field_name("definition")
                    .map(|n| n.kind() == "function_definition")
                    .unwrap_or(false))
        {
            let func_node = if child.kind() == "function_definition" {
                child
            } else {
                child
                    .child_by_field_name("definition")
                    .expect("decorated_definition must have definition")
            };

            if let Some(name_node) = func_node.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(source_bytes)
                    .unwrap_or("<unknown>")
                    .to_string();

                let (start_line, end_line) =
                    expand_range_for_trivia(child, root_node, &TriviaConfig::python());

                containers.push(ContainerWithMembers {
                    container: Container {
                        kind: ContainerKind::Function,
                        name,
                        start_line,
                        end_line,
                    },
                    members: Vec::new(), // Functions don't have members
                });
            }
        }
    }

    containers
}

/// Extract members (properties and methods) from a Kotlin class/object/interface.
#[cfg(feature = "tree-sitter")]
pub fn extract_kotlin_members(
    body_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut members = Vec::new();
    let mut cursor = body_node.walk();

    for item in body_node.children(&mut cursor) {
        match item.kind() {
            "property_declaration" => {
                if let Some(name_node) = item.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) =
                        expand_range_for_trivia(item, body_node, &TriviaConfig::kotlin());

                    members.push(Member {
                        kind: MemberKind::Property,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
            "function_declaration" => {
                if let Some(name_node) = item.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) =
                        expand_range_for_trivia(item, body_node, &TriviaConfig::kotlin());

                    members.push(Member {
                        kind: MemberKind::Method,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
            _ => {}
        }
    }

    members
}

/// Extract containers with their members from a parsed Kotlin file.
#[cfg(feature = "tree-sitter")]
pub fn extract_kotlin_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
    let mut containers = Vec::new();
    let root_node = parsed.tree.root_node();
    let source_bytes = parsed.source.as_bytes();

    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    // Find class_body by kind, not by field name
                    let mut cursor2 = child.walk();
                    let class_body = child.children(&mut cursor2)
                        .find(|c| c.kind() == "class_body");
                    let members = class_body
                        .map(|body| extract_kotlin_members(body, source_bytes))
                        .unwrap_or_default();

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::kotlin());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Class,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            "object_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    // Find class_body by kind, not by field name
                    let mut cursor2 = child.walk();
                    let class_body = child.children(&mut cursor2)
                        .find(|c| c.kind() == "class_body");
                    let members = class_body
                        .map(|body| extract_kotlin_members(body, source_bytes))
                        .unwrap_or_default();

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::kotlin());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Object,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            "interface_declaration" | "annotation_class" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    // Find class_body by kind, not by field name
                    let mut cursor2 = child.walk();
                    let class_body = child.children(&mut cursor2)
                        .find(|c| c.kind() == "class_body");
                    let members = class_body
                        .map(|body| extract_kotlin_members(body, source_bytes))
                        .unwrap_or_default();

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::kotlin());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Interface,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            "function_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::kotlin());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Function,
                            name,
                            start_line,
                            end_line,
                        },
                        members: Vec::new(),
                    });
                }
            }
            _ => {}
        }
    }

    containers
}

/// Extract members (fields and methods) from a Java class/interface/enum body.
#[cfg(feature = "tree-sitter")]
pub fn extract_java_members(
    body_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut members = Vec::new();
    let mut cursor = body_node.walk();

    for item in body_node.children(&mut cursor) {
        match item.kind() {
            "field_declaration" => {
                // Java fields can declare multiple variables, extract each
                let mut field_cursor = item.walk();
                for field_child in item.children(&mut field_cursor) {
                    if field_child.kind() == "variable_declarator" {
                        if let Some(name_node) = field_child.child_by_field_name("name") {
                            let name = name_node
                                .utf8_text(source_bytes)
                                .unwrap_or("<unknown>")
                                .to_string();

                            let (start_line, end_line) =
                                expand_range_for_trivia(item, body_node, &TriviaConfig::java());

                            members.push(Member {
                                kind: MemberKind::Field,
                                name,
                                start_line,
                                end_line,
                            });
                            break; // Only take first variable declarator for the whole field declaration
                        }
                    }
                }
            }
            "method_declaration" | "constructor_declaration" => {
                if let Some(name_node) = item.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) =
                        expand_range_for_trivia(item, body_node, &TriviaConfig::java());

                    members.push(Member {
                        kind: MemberKind::Method,
                        name,
                        start_line,
                        end_line,
                    });
                }
            }
            _ => {}
        }
    }

    members
}

/// Extract containers with their members from a parsed Java file.
#[cfg(feature = "tree-sitter")]
pub fn extract_java_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
    let mut containers = Vec::new();
    let root_node = parsed.tree.root_node();
    let source_bytes = parsed.source.as_bytes();

    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_java_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Class,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            "interface_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_java_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Interface,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            "enum_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_java_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Enum,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
                    });
                }
            }
            _ => {}
        }
    }

    containers
}

/// Extract containers with their members from a parsed Rust file.
///
/// Returns a vector of containers (structs, impls, functions) with their associated
/// members (fields, methods). Line ranges are expanded to include attributes and comments.
#[cfg(feature = "tree-sitter")]
pub fn extract_rust_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
    let mut containers = Vec::new();
    let root_node = parsed.tree.root_node();
    let source_bytes = parsed.source.as_bytes();

    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        match child.kind() {
            "struct_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let fields = extract_struct_fields(child, source_bytes);
                    let (start_line, end_line) = expand_range_for_attributes_and_comments(child, root_node);

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Struct,
                            name,
                            start_line,
                            end_line,
                        },
                        members: fields,
                    });
                }
            }
            "impl_item" => {
                let type_node = child.child_by_field_name("type");
                let trait_node = child.child_by_field_name("trait");

                if let Some(type_node) = type_node {
                    let type_name = type_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let trait_name = trait_node.and_then(|node| {
                        node.utf8_text(source_bytes).ok().map(|s| s.to_string())
                    });

                    let methods = extract_impl_methods(child, source_bytes);
                    let (start_line, end_line) = expand_range_for_attributes_and_comments(child, root_node);

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Impl { trait_name },
                            name: type_name,
                            start_line,
                            end_line,
                        },
                        members: methods,
                    });
                }
            }
            "function_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) = expand_range_for_attributes_and_comments(child, root_node);

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Function,
                            name,
                            start_line,
                            end_line,
                        },
                        members: Vec::new(), // Functions don't have members
                    });
                }
            }
            _ => {}
        }
    }

    containers
}

/// A semantic container with its extracted members.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerWithMembers {
    /// The container information
    pub container: Container,
    /// The members within this container (fields for structs, methods for impls)
    pub members: Vec<Member>,
}

/// Errors that can occur during semantic parsing.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// Language detection failed
    #[error("unsupported file type for semantic parsing")]
    UnsupportedLanguage,

    /// Parser setup failed
    #[error("failed to initialize parser for {language}: {error}")]
    ParserSetup {
        /// The language being parsed
        language: &'static str,
        /// The error message
        error: String,
    },

    /// Parsing failed
    #[error("tree-sitter parsing failed")]
    ParseFailed,

    /// Syntax error in source code
    #[error("syntax error in source code")]
    SyntaxError,

    /// Timeout during parsing
    #[error("parsing timeout")]
    Timeout,
}

/// Try to enhance a File with semantic containers by parsing the file contents.
///
/// This is the main integration point for scm-diff-editor. Call this after
/// creating a File with sections to optionally populate the `containers` field.
///
/// If semantic parsing fails for any reason, the File is returned unchanged
/// (with empty containers field), allowing graceful fallback to diff-first navigation.
///
/// # Example (for scm-diff-editor integration)
///
/// ```ignore
/// let mut file = File {
///     path: Cow::Owned(right_display_path),
///     file_mode: left_file_mode,
///     sections,
///     #[cfg(feature = "tree-sitter")]
///     containers: None,
/// };
///
/// #[cfg(feature = "tree-sitter")]
/// {
///     file = scm_record::semantic::try_add_semantic_containers(
///         file,
///         &left_contents,  // old source
///         &right_contents, // new source
///     );
/// }
/// ```
#[cfg(feature = "tree-sitter")]
/// Represents the line range of a section in the new file.
#[derive(Debug, Clone)]
struct SectionLineRange {
    /// Index of this section in the original sections Vec
    section_index: usize,
    /// Starting line number in the new file (0-indexed)
    start_line: usize,
    /// Ending line number in the new file (exclusive, so end_line = start_line + line_count)
    end_line: usize,
}

/// Calculate the line ranges for each section in the new file.
///
/// Tracks which lines each section occupies in the new file by:
/// - Counting all lines in Unchanged sections (exist in both files)
/// - Counting only Added lines in Changed sections (only in new file)
/// - Ignoring Removed lines (only in old file)
fn calculate_section_line_ranges(sections: &[crate::Section<'_>]) -> Vec<SectionLineRange> {
    use crate::{ChangeType, Section};

    let mut ranges = Vec::new();
    let mut current_line = 0;

    for (section_index, section) in sections.iter().enumerate() {
        let start_line = current_line;

        match section {
            Section::Unchanged { lines } => {
                // Unchanged lines exist in both files at the same positions
                current_line += lines.len();
            }
            Section::Changed { lines } => {
                // Count only Added lines (they're in the new file)
                let added_count = lines
                    .iter()
                    .filter(|l| l.change_type == ChangeType::Added)
                    .count();
                current_line += added_count;
            }
            Section::FileMode { .. } | Section::Binary { .. } => {
                // These don't represent actual file content lines
                continue;
            }
        }

        let end_line = current_line;

        // Only add ranges for sections that have lines
        if end_line > start_line {
            ranges.push(SectionLineRange {
                section_index,
                start_line,
                end_line,
            });
        }
    }

    ranges
}

/// Filter sections that overlap with the given line range.
///
/// A section overlaps if any part of its line range intersects with [start_line, end_line).
/// Returns a Vec of section indices that fall within or partially overlap the range.
fn filter_section_indices_by_range(
    section_ranges: &[SectionLineRange],
    start_line: usize,
    end_line: usize,
) -> Vec<usize> {
    section_ranges
        .iter()
        .filter(|range| {
            // Check if ranges overlap: [range.start_line, range.end_line) and [start_line, end_line)
            // Ranges overlap if: range.start_line < end_line AND start_line < range.end_line
            range.start_line < end_line && start_line < range.end_line
        })
        .map(|range| range.section_index)
        .collect()
}

/// Attempts to enhance a File with semantic containers by parsing source code.
///
/// This function takes a File with traditional diff-first sections and attempts to
/// reorganize it into a semantic-first structure based on code syntax (containers
/// like structs/impls/functions and members like fields/methods).
///
/// If semantic parsing succeeds:
/// - `file.containers` is set to `Some(Vec<SemanticContainer>)` with the semantic hierarchy
/// - `file.sections` is kept unchanged for backwards compatibility with existing UI
/// - Sections are ALSO distributed into containers/members for future semantic-aware UI
///
/// If semantic parsing fails (unsupported language, parse error, or no containers found):
/// - `file.containers` remains `None`
/// - `file.sections` is left unchanged for traditional diff-first navigation
///
/// # Arguments
///
/// * `file` - The File to enhance with semantic information
/// * `old_source` - The source code of the old version (for future matching)
/// * `new_source` - The source code of the new version (used for extraction)
///
/// # Returns
///
/// The enhanced File with semantic containers if parsing succeeded, otherwise unchanged.
///
/// # Example
///
/// ```no_run
/// use scm_record::semantic::try_add_semantic_containers;
/// # use scm_record::File;
/// # use std::borrow::Cow;
/// # use std::path::Path;
/// # let file = File {
/// #     old_path: None,
/// #     path: Cow::Borrowed(Path::new("foo.rs")),
/// #     file_mode: scm_record::FileMode::FILE_DEFAULT,
/// #     sections: vec![],
/// #     containers: None,
/// # };
/// # let old_source = "";
/// # let new_source = "";
///
/// let enhanced_file = try_add_semantic_containers(
///     file,
///     old_source,
///     new_source,
/// );
/// ```
pub fn try_add_semantic_containers<'a>(
    mut file: crate::File<'a>,
    old_source: &str,
    new_source: &str,
) -> crate::File<'a> {
    use crate::{SemanticContainer, SemanticMember};

    // Detect language from file path
    let language = match SupportedLanguage::from_path(&file.path) {
        Some(lang) => lang,
        None => return file, // Unsupported language, return unchanged
    };

    // Parse both versions
    let (_old_parsed, new_parsed) = match parse_file_versions(language, old_source, new_source) {
        Ok(parsed) => parsed,
        Err(_) => return file, // Parse failed, fall back
    };

    // TODO: Implement rename detection by matching containers between old_parsed and new_parsed.
    // This would allow us to detect when a function/class/etc. is renamed and show it as a
    // modification rather than a deletion + addition. Matching could use similarity metrics
    // on container structure, member names, and/or content.

    // Extract containers with members from the new version (language-specific)
    let containers_with_members = match language {
        SupportedLanguage::Rust => rust::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Python => python::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Kotlin => kotlin::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Java => java::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Hcl => hcl::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Markdown => markdown::extract_containers_with_members(&new_parsed),
        SupportedLanguage::Yaml => yaml::extract_containers_with_members(&new_parsed),
    };

    // Build semantic containers with section mapping
    // Calculate line ranges and build section assignments upfront
    let section_ranges = calculate_section_line_ranges(&file.sections);

    // Build a map of (container_index, member_index_option) -> Vec<section_indices>
    // This separates the borrowing from the section building
    let mut section_assignments: Vec<(usize, Option<usize>, Vec<usize>)> = Vec::new();

    for (container_idx, container_with_members) in containers_with_members.iter().enumerate() {
        let ContainerWithMembers { container, members } = container_with_members;

        // For functions (no members), assign sections directly to the container
        if matches!(container.kind, ContainerKind::Function) {
            let section_indices = filter_section_indices_by_range(
                &section_ranges,
                container.start_line,
                container.end_line,
            );
            section_assignments.push((container_idx, None, section_indices));
        } else {
            // For structs and impls, assign sections to each member
            for (member_idx, member) in members.iter().enumerate() {
                let section_indices = filter_section_indices_by_range(
                    &section_ranges,
                    member.start_line,
                    member.end_line,
                );
                section_assignments.push((container_idx, Some(member_idx), section_indices));
            }
        }
    }

    // Keep file.sections for backwards compatibility with existing UI
    // The UI currently only understands sections, not semantic containers
    // Future work: Update UI to render semantic hierarchy from file.containers

    // Helper to check if sections contain editable changes
    let has_editable_sections = |indices: &[usize]| -> bool {
        indices.iter().any(|&idx| {
            file.sections
                .get(idx)
                .map(|s| s.is_editable())
                .unwrap_or(false)
        })
    };

    // Now build semantic containers using the pre-computed section assignments
    let semantic_containers: Vec<SemanticContainer> = containers_with_members
        .into_iter()
        .enumerate()
        .filter_map(|(container_idx, c)| {
            let ContainerWithMembers { container, members } = c;

            let container = match container.kind {
                ContainerKind::Struct => {
                    let fields: Vec<_> = members
                        .into_iter()
                        .enumerate()
                        .filter_map(|(member_idx, m)| {
                            let section_indices = section_assignments
                                .iter()
                                .find(|(c_idx, m_idx, _)| {
                                    *c_idx == container_idx && *m_idx == Some(member_idx)
                                })
                                .map(|(_, _, indices)| indices.clone())
                                .unwrap_or_default();

                            // Filter out members with no editable changes
                            if !has_editable_sections(&section_indices) {
                                return None;
                            }

                            // Keep ALL sections (including context) for display
                            Some(SemanticMember::Field {
                                name: m.name,
                                section_indices,
                                is_checked: false,
                                is_partial: false,
                            })
                        })
                        .collect();

                    // Filter out structs with no fields that have changes
                    if fields.is_empty() {
                        return None;
                    }

                    SemanticContainer::Struct {
                        name: container.name,
                        fields,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Impl { trait_name } => {
                    let methods: Vec<_> = members
                        .into_iter()
                        .enumerate()
                        .filter_map(|(member_idx, m)| {
                            let section_indices = section_assignments
                                .iter()
                                .find(|(c_idx, m_idx, _)| {
                                    *c_idx == container_idx && *m_idx == Some(member_idx)
                                })
                                .map(|(_, _, indices)| indices.clone())
                                .unwrap_or_default();

                            // Filter out methods with no editable changes
                            if !has_editable_sections(&section_indices) {
                                return None;
                            }

                            // Keep ALL sections (including context) for display
                            Some(SemanticMember::Method {
                                name: m.name,
                                section_indices,
                                is_checked: false,
                                is_partial: false,
                            })
                        })
                        .collect();

                    // Filter out impls with no methods that have changes
                    if methods.is_empty() {
                        return None;
                    }

                    SemanticContainer::Impl {
                        type_name: container.name,
                        trait_name,
                        methods,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Function => {
                    let section_indices = section_assignments
                        .iter()
                        .find(|(c_idx, m_idx, _)| *c_idx == container_idx && m_idx.is_none())
                        .map(|(_, _, indices)| indices.clone())
                        .unwrap_or_default();

                    // Filter out functions with no editable changes
                    if !has_editable_sections(&section_indices) {
                        return None;
                    }

                    // Keep ALL sections (including context) for display
                    SemanticContainer::Function {
                        name: container.name,
                        section_indices,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Class => {
                    let members: Vec<_> = members
                        .into_iter()
                        .enumerate()
                        .filter_map(|(member_idx, m)| {
                            let section_indices = section_assignments
                                .iter()
                                .find(|(c_idx, m_idx, _)| {
                                    *c_idx == container_idx && *m_idx == Some(member_idx)
                                })
                                .map(|(_, _, indices)| indices.clone())
                                .unwrap_or_default();

                            // Filter out members with no editable changes
                            if !has_editable_sections(&section_indices) {
                                return None;
                            }

                            // Determine member type based on MemberKind
                            match m.kind {
                                MemberKind::Field | MemberKind::Property => Some(SemanticMember::Field {
                                    name: m.name,
                                    section_indices,
                                    is_checked: false,
                                    is_partial: false,
                                }),
                                MemberKind::Method => Some(SemanticMember::Method {
                                    name: m.name,
                                    section_indices,
                                    is_checked: false,
                                    is_partial: false,
                                }),
                            }
                        })
                        .collect();

                    // Filter out classes with no members that have changes
                    if members.is_empty() {
                        return None;
                    }

                    SemanticContainer::Class {
                        name: container.name,
                        members,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Interface => {
                    let methods: Vec<_> = members
                        .into_iter()
                        .enumerate()
                        .filter_map(|(member_idx, m)| {
                            let section_indices = section_assignments
                                .iter()
                                .find(|(c_idx, m_idx, _)| {
                                    *c_idx == container_idx && *m_idx == Some(member_idx)
                                })
                                .map(|(_, _, indices)| indices.clone())
                                .unwrap_or_default();

                            // Filter out methods with no editable changes
                            if !has_editable_sections(&section_indices) {
                                return None;
                            }

                            Some(SemanticMember::Method {
                                name: m.name,
                                section_indices,
                                is_checked: false,
                                is_partial: false,
                            })
                        })
                        .collect();

                    // Filter out interfaces with no methods that have changes
                    if methods.is_empty() {
                        return None;
                    }

                    SemanticContainer::Interface {
                        name: container.name,
                        methods,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Enum => {
                    let section_indices = section_assignments
                        .iter()
                        .find(|(c_idx, m_idx, _)| *c_idx == container_idx && m_idx.is_none())
                        .map(|(_, _, indices)| indices.clone())
                        .unwrap_or_default();

                    // Filter out enums with no editable changes
                    if !has_editable_sections(&section_indices) {
                        return None;
                    }

                    SemanticContainer::Enum {
                        name: container.name,
                        section_indices,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Object => {
                    let section_indices = section_assignments
                        .iter()
                        .find(|(c_idx, m_idx, _)| *c_idx == container_idx && m_idx.is_none())
                        .map(|(_, _, indices)| indices.clone())
                        .unwrap_or_default();

                    // Filter out objects with no editable changes
                    if !has_editable_sections(&section_indices) {
                        return None;
                    }

                    SemanticContainer::Object {
                        name: container.name,
                        section_indices,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Module => {
                    let section_indices = section_assignments
                        .iter()
                        .find(|(c_idx, m_idx, _)| *c_idx == container_idx && m_idx.is_none())
                        .map(|(_, _, indices)| indices.clone())
                        .unwrap_or_default();

                    // Filter out modules with no editable changes
                    if !has_editable_sections(&section_indices) {
                        return None;
                    }

                    SemanticContainer::Module {
                        name: container.name,
                        section_indices,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                ContainerKind::Section { level } => {
                    let section_indices = section_assignments
                        .iter()
                        .find(|(c_idx, m_idx, _)| *c_idx == container_idx && m_idx.is_none())
                        .map(|(_, _, indices)| indices.clone())
                        .unwrap_or_default();

                    // Filter out sections with no editable changes
                    if !has_editable_sections(&section_indices) {
                        return None;
                    }

                    SemanticContainer::Section {
                        name: container.name,
                        level,
                        section_indices,
                        is_checked: false,
                        is_partial: false,
                    }
                }
                // HCL and YAML container kinds not yet supported in UI
                ContainerKind::Resource { .. }
                | ContainerKind::DataSource { .. }
                | ContainerKind::Variable
                | ContainerKind::Output => {
                    // TODO: Implement UI display for HCL/YAML container kinds
                    return None;
                }
            };

            Some(container)
        })
        .collect();

    // Use the semantic containers if we successfully built any
    if !semantic_containers.is_empty() {
        file.containers = Some(semantic_containers);
    }

    file
}

// Language-specific modules
#[cfg(feature = "tree-sitter")]
pub mod rust;
#[cfg(feature = "tree-sitter")]
pub mod python;
#[cfg(feature = "tree-sitter")]
pub mod kotlin;
#[cfg(feature = "tree-sitter")]
pub mod java;
#[cfg(feature = "tree-sitter")]
pub mod hcl;
#[cfg(feature = "tree-sitter")]
pub mod markdown;
#[cfg(feature = "tree-sitter")]
pub mod yaml;
