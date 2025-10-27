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

/// Information about a Rust container (struct, impl, function) extracted from the AST.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustContainer {
    /// The type of container
    pub kind: RustContainerKind,
    /// The name of the container
    pub name: String,
    /// Start line number (0-indexed)
    pub start_line: usize,
    /// End line number (0-indexed)
    pub end_line: usize,
}

/// The kind of Rust container.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustContainerKind {
    /// A struct definition
    Struct,
    /// An impl block
    Impl {
        /// The trait being implemented, if any
        trait_name: Option<String>,
    },
    /// A top-level function
    Function,
}

/// Extract Rust containers from a parsed syntax tree.
#[cfg(feature = "tree-sitter")]
pub fn extract_rust_containers(parsed: &ParsedFile) -> Vec<RustContainer> {
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

                    containers.push(RustContainer {
                        kind: RustContainerKind::Struct,
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

                    containers.push(RustContainer {
                        kind: RustContainerKind::Impl { trait_name },
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

                    containers.push(RustContainer {
                        kind: RustContainerKind::Function,
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

/// Information about a Rust member (field or method) extracted from the AST.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustMember {
    /// The type of member
    pub kind: RustMemberKind,
    /// The name of the member
    pub name: String,
    /// Start line number (0-indexed)
    pub start_line: usize,
    /// End line number (0-indexed)
    pub end_line: usize,
}

/// The kind of Rust member.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustMemberKind {
    /// A struct field
    Field,
    /// A method in an impl block
    Method,
}

/// Extract struct fields from a struct definition node.
#[cfg(feature = "tree-sitter")]
pub fn extract_struct_fields(
    struct_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<RustMember> {
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

                    fields.push(RustMember {
                        kind: RustMemberKind::Field,
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
) -> Vec<RustMember> {
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

                    methods.push(RustMember {
                        kind: RustMemberKind::Method,
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

/// Expands a node's line range to include preceding attributes, comments, and whitespace.
///
/// This ensures that when we group sections by semantic structure, we include the full
/// declaration including doc comments, attributes, and surrounding whitespace.
#[cfg(feature = "tree-sitter")]
fn expand_range_for_attributes_and_comments(
    node: tree_sitter::Node,
    parent: tree_sitter::Node,
) -> (usize, usize) {
    let mut start_line = node.start_position().row;
    let end_line = node.end_position().row;

    // Walk backwards through siblings to find attributes and comments
    let mut cursor = parent.walk();
    let siblings: Vec<_> = parent.children(&mut cursor).collect();

    if let Some(node_index) = siblings.iter().position(|n| n.id() == node.id()) {
        // Look at all previous siblings in reverse order
        for sibling in siblings[..node_index].iter().rev() {
            match sibling.kind() {
                // Include attribute items like #[test], #[cfg(...)]
                "attribute_item" => {
                    start_line = start_line.min(sibling.start_position().row);
                }
                // Include doc comments ///
                "line_comment" => {
                    let sibling_line = sibling.start_position().row;
                    // Only include if it's adjacent or within 1 line
                    if start_line.saturating_sub(sibling_line) <= 1 {
                        start_line = sibling_line;
                    } else {
                        break; // Stop if there's a gap
                    }
                }
                // Stop at non-comment/non-attribute siblings
                _ if !sibling.kind().is_empty() => break,
                _ => {}
            }
        }
    }

    (start_line, end_line)
}

/// Extract containers with their members from a parsed Rust file.
///
/// Returns a vector of containers (structs, impls, functions) with their associated
/// members (fields, methods). Line ranges are expanded to include attributes and comments.
#[cfg(feature = "tree-sitter")]
pub fn extract_rust_containers_with_members(parsed: &ParsedFile) -> Vec<RustContainerWithMembers> {
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

                    containers.push(RustContainerWithMembers {
                        container: RustContainer {
                            kind: RustContainerKind::Struct,
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

                    containers.push(RustContainerWithMembers {
                        container: RustContainer {
                            kind: RustContainerKind::Impl { trait_name },
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

                    containers.push(RustContainerWithMembers {
                        container: RustContainer {
                            kind: RustContainerKind::Function,
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

/// A Rust container with its extracted members.
#[cfg(feature = "tree-sitter")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustContainerWithMembers {
    /// The container information
    pub container: RustContainer,
    /// The members within this container (fields for structs, methods for impls)
    pub members: Vec<RustMember>,
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

    // Only Rust is implemented for now
    if !matches!(language, SupportedLanguage::Rust) {
        return file;
    }

    // Parse both versions
    let (_old_parsed, new_parsed) = match parse_file_versions(language, old_source, new_source) {
        Ok(parsed) => parsed,
        Err(_) => return file, // Parse failed, fall back
    };

    // Extract containers with members from the new version
    let containers_with_members = extract_rust_containers_with_members(&new_parsed);

    // Build semantic containers with section mapping
    // Calculate line ranges and build section assignments upfront
    let section_ranges = calculate_section_line_ranges(&file.sections);

    // Build a map of (container_index, member_index_option) -> Vec<section_indices>
    // This separates the borrowing from the section building
    let mut section_assignments: Vec<(usize, Option<usize>, Vec<usize>)> = Vec::new();

    for (container_idx, container_with_members) in containers_with_members.iter().enumerate() {
        let RustContainerWithMembers { container, members } = container_with_members;

        // For functions (no members), assign sections directly to the container
        if matches!(container.kind, RustContainerKind::Function) {
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
            let RustContainerWithMembers { container, members } = c;

            let container = match container.kind {
                RustContainerKind::Struct => {
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
                RustContainerKind::Impl { trait_name } => {
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
                RustContainerKind::Function => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_detection_rust() {
        let path = PathBuf::from("test.rs");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Rust));
    }

    #[test]
    fn test_language_detection_kotlin() {
        let path = PathBuf::from("test.kt");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Kotlin));

        let path = PathBuf::from("test.kts");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Kotlin));
    }

    #[test]
    fn test_language_detection_java() {
        let path = PathBuf::from("test.java");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Java));
    }

    #[test]
    fn test_language_detection_hcl() {
        let path = PathBuf::from("main.tf");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Hcl));

        let path = PathBuf::from("test.hcl");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Hcl));
    }

    #[test]
    fn test_language_detection_python() {
        let path = PathBuf::from("test.py");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Python));
    }

    #[test]
    fn test_language_detection_markdown() {
        let path = PathBuf::from("README.md");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Markdown));
    }

    #[test]
    fn test_language_detection_yaml() {
        let path = PathBuf::from("config.yaml");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Yaml));

        let path = PathBuf::from("config.yml");
        assert_eq!(SupportedLanguage::from_path(&path), Some(SupportedLanguage::Yaml));
    }

    #[test]
    fn test_language_detection_unsupported() {
        let path = PathBuf::from("test.txt");
        assert_eq!(SupportedLanguage::from_path(&path), None);
    }

    #[test]
    fn test_language_names() {
        assert_eq!(SupportedLanguage::Rust.name(), "Rust");
        assert_eq!(SupportedLanguage::Kotlin.name(), "Kotlin");
        assert_eq!(SupportedLanguage::Java.name(), "Java");
        assert_eq!(SupportedLanguage::Hcl.name(), "HCL");
        assert_eq!(SupportedLanguage::Python.name(), "Python");
        assert_eq!(SupportedLanguage::Markdown.name(), "Markdown");
        assert_eq!(SupportedLanguage::Yaml.name(), "YAML");
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_parser_creation_rust() {
        let result = create_parser(SupportedLanguage::Rust);
        assert!(result.is_ok());
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_simple_rust_parse() {
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let source = "fn main() { println!(\"Hello, world!\"); }";
        let result = parse_source(&mut parser, source);
        assert!(result.is_ok());
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_rust_struct() {
        let source = r#"
struct Point {
    x: i32,
    y: i32,
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "Point");
        assert!(matches!(
            containers[0].kind,
            RustContainerKind::Struct
        ));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_rust_impl() {
        let source = r#"
impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "Point");
        assert!(matches!(
            containers[0].kind,
            RustContainerKind::Impl { trait_name: None }
        ));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_rust_trait_impl() {
        let source = r#"
impl Display for Point {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "Point");
        if let RustContainerKind::Impl { trait_name } = &containers[0].kind {
            assert_eq!(trait_name.as_deref(), Some("Display"));
        } else {
            panic!("Expected Impl with trait");
        }
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_rust_function() {
        let source = r#"
fn calculate_distance(p1: &Point, p2: &Point) -> f64 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    ((dx * dx + dy * dy) as f64).sqrt()
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "calculate_distance");
        assert!(matches!(
            containers[0].kind,
            RustContainerKind::Function
        ));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_mixed_rust_containers() {
        let source = r#"
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

fn origin() -> Point {
    Point { x: 0, y: 0 }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers(&parsed);
        assert_eq!(containers.len(), 3);

        assert_eq!(containers[0].name, "Point");
        assert!(matches!(containers[0].kind, RustContainerKind::Struct));

        assert_eq!(containers[1].name, "Point");
        assert!(matches!(
            containers[1].kind,
            RustContainerKind::Impl { trait_name: None }
        ));

        assert_eq!(containers[2].name, "origin");
        assert!(matches!(containers[2].kind, RustContainerKind::Function));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_parse_file_versions() {
        let old_source = r#"
struct Point {
    x: i32,
}
"#;
        let new_source = r#"
struct Point {
    x: i32,
    y: i32,
}
"#;

        let result = parse_file_versions(SupportedLanguage::Rust, old_source, new_source);
        assert!(result.is_ok());

        let (old_parsed, new_parsed) = result.unwrap();
        assert_eq!(old_parsed.source, old_source);
        assert_eq!(new_parsed.source, new_source);

        let old_containers = extract_rust_containers(&old_parsed);
        let new_containers = extract_rust_containers(&new_parsed);

        assert_eq!(old_containers.len(), 1);
        assert_eq!(new_containers.len(), 1);
        assert_eq!(old_containers[0].name, "Point");
        assert_eq!(new_containers[0].name, "Point");
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_struct_fields() {
        let source = r#"
struct Point {
    x: i32,
    y: i32,
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.container.name, "Point");
        assert_eq!(container.members.len(), 2);

        assert_eq!(container.members[0].name, "x");
        assert!(matches!(container.members[0].kind, RustMemberKind::Field));

        assert_eq!(container.members[1].name, "y");
        assert!(matches!(container.members[1].kind, RustMemberKind::Field));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_impl_methods() {
        let source = r#"
impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    fn distance(&self, other: &Point) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx * dx + dy * dy) as f64).sqrt()
    }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.container.name, "Point");
        assert_eq!(container.members.len(), 2);

        assert_eq!(container.members[0].name, "new");
        assert!(matches!(
            container.members[0].kind,
            RustMemberKind::Method
        ));

        assert_eq!(container.members[1].name, "distance");
        assert!(matches!(
            container.members[1].kind,
            RustMemberKind::Method
        ));
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_extract_complete_struct_with_impl() {
        let source = r#"
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    fn origin() -> Self {
        Point { x: 0, y: 0 }
    }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_rust_containers_with_members(&parsed);
        assert_eq!(containers.len(), 2);

        // Struct with fields
        assert_eq!(containers[0].container.name, "Point");
        assert!(matches!(
            containers[0].container.kind,
            RustContainerKind::Struct
        ));
        assert_eq!(containers[0].members.len(), 2);
        assert_eq!(containers[0].members[0].name, "x");
        assert_eq!(containers[0].members[1].name, "y");

        // Impl with methods
        assert_eq!(containers[1].container.name, "Point");
        assert!(matches!(
            containers[1].container.kind,
            RustContainerKind::Impl { trait_name: None }
        ));
        assert_eq!(containers[1].members.len(), 2);
        assert_eq!(containers[1].members[0].name, "new");
        assert_eq!(containers[1].members[1].name, "origin");
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_try_add_semantic_containers() {
        use crate::{ChangeType, File, FileMode, Section, SectionChangedLine, SemanticContainer};
        use std::borrow::Cow;

        // Simpler test: just a function with changes
        let old_source = r#"
fn hello() {
    println!("old");
}

fn world() {
    println!("same");
}
"#;
        let new_source = r#"
fn hello() {
    println!("new");
}

fn world() {
    println!("same");
}
"#;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.rs")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: vec![
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("\n"),
                        Cow::Borrowed("fn hello() {\n"),
                    ],
                },
                Section::Changed {
                    lines: vec![
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Removed,
                            line: Cow::Borrowed("    println!(\"old\");\n"),
                        },
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Added,
                            line: Cow::Borrowed("    println!(\"new\");\n"),
                        },
                    ],
                },
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("}\n"),
                        Cow::Borrowed("\n"),
                        Cow::Borrowed("fn world() {\n"),
                        Cow::Borrowed("    println!(\"same\");\n"),
                        Cow::Borrowed("}\n"),
                    ],
                },
            ],
            containers: None,
        };

        let enhanced_file = try_add_semantic_containers(file, old_source, new_source);

        assert!(enhanced_file.containers.is_some());
        let containers = enhanced_file.containers.unwrap();
        // Should only have hello() function, world() is filtered out (no editable changes)
        assert_eq!(containers.len(), 1);

        // Check function
        match &containers[0] {
            SemanticContainer::Function {
                name,
                section_indices,
                is_checked: _,
                is_partial: _,
            } => {
                assert_eq!(name, "hello");
                // Should have sections that fall within the function's line range
                assert!(!section_indices.is_empty());
                // At minimum, should have the Changed section
                assert!(section_indices.contains(&1));
            }
            _ => panic!("Expected Function container"),
        }
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_try_add_semantic_containers_unsupported_language() {
        use crate::{File, FileMode};
        use std::borrow::Cow;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.txt")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: Vec::new(),
            containers: None,
        };

        let enhanced_file = try_add_semantic_containers(file, "old", "new");

        // Should return unchanged for unsupported language
        assert!(enhanced_file.containers.is_none());
    }

    #[test]
    fn test_filter_section_indices_by_range_exact_match() {
        let section_ranges = vec![
            SectionLineRange {
                section_index: 0,
                start_line: 0,
                end_line: 5,
            },
            SectionLineRange {
                section_index: 1,
                start_line: 10,
                end_line: 15,
            },
            SectionLineRange {
                section_index: 2,
                start_line: 20,
                end_line: 25,
            },
        ];

        let indices = filter_section_indices_by_range(&section_ranges, 10, 15);
        assert_eq!(indices, vec![1]);
    }

    #[test]
    fn test_filter_section_indices_by_range_overlap() {
        let section_ranges = vec![
            SectionLineRange {
                section_index: 0,
                start_line: 0,
                end_line: 10,
            },
            SectionLineRange {
                section_index: 1,
                start_line: 8,
                end_line: 15,
            },
            SectionLineRange {
                section_index: 2,
                start_line: 20,
                end_line: 25,
            },
        ];

        // Range [5, 12) should overlap with sections 0 and 1
        let indices = filter_section_indices_by_range(&section_ranges, 5, 12);
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn test_filter_section_indices_by_range_no_overlap() {
        let section_ranges = vec![
            SectionLineRange {
                section_index: 0,
                start_line: 0,
                end_line: 5,
            },
            SectionLineRange {
                section_index: 1,
                start_line: 10,
                end_line: 15,
            },
        ];

        // Range [6, 9) doesn't overlap with any section
        let indices = filter_section_indices_by_range(&section_ranges, 6, 9);
        assert_eq!(indices, Vec::<usize>::new());
    }

    #[test]
    fn test_filter_section_indices_by_range_contains_all() {
        let section_ranges = vec![
            SectionLineRange {
                section_index: 0,
                start_line: 5,
                end_line: 10,
            },
            SectionLineRange {
                section_index: 1,
                start_line: 15,
                end_line: 20,
            },
            SectionLineRange {
                section_index: 2,
                start_line: 25,
                end_line: 30,
            },
        ];

        // Range [0, 100) contains all sections
        let indices = filter_section_indices_by_range(&section_ranges, 0, 100);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_filter_section_indices_by_range_partial_overlap_start() {
        let section_ranges = vec![SectionLineRange {
            section_index: 0,
            start_line: 10,
            end_line: 20,
        }];

        // Range [5, 15) overlaps with section at the start
        let indices = filter_section_indices_by_range(&section_ranges, 5, 15);
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_filter_section_indices_by_range_partial_overlap_end() {
        let section_ranges = vec![SectionLineRange {
            section_index: 0,
            start_line: 10,
            end_line: 20,
        }];

        // Range [15, 25) overlaps with section at the end
        let indices = filter_section_indices_by_range(&section_ranges, 15, 25);
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_calculate_section_line_ranges() {
        use crate::{ChangeType, Section, SectionChangedLine};
        use std::borrow::Cow;

        let sections = vec![
            Section::Unchanged {
                lines: vec![
                    Cow::Borrowed("line1\n"),
                    Cow::Borrowed("line2\n"),
                    Cow::Borrowed("line3\n"),
                ],
            },
            Section::Changed {
                lines: vec![
                    SectionChangedLine {
                        is_checked: false,
                        change_type: ChangeType::Removed,
                        line: Cow::Borrowed("old\n"),
                    },
                    SectionChangedLine {
                        is_checked: false,
                        change_type: ChangeType::Added,
                        line: Cow::Borrowed("new1\n"),
                    },
                    SectionChangedLine {
                        is_checked: false,
                        change_type: ChangeType::Added,
                        line: Cow::Borrowed("new2\n"),
                    },
                ],
            },
            Section::Unchanged {
                lines: vec![Cow::Borrowed("line4\n"), Cow::Borrowed("line5\n")],
            },
        ];

        let ranges = calculate_section_line_ranges(&sections);

        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].section_index, 0);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 3); // 3 lines

        assert_eq!(ranges[1].section_index, 1);
        assert_eq!(ranges[1].start_line, 3);
        assert_eq!(ranges[1].end_line, 5); // 2 added lines (removed doesn't count)

        assert_eq!(ranges[2].section_index, 2);
        assert_eq!(ranges[2].start_line, 5);
        assert_eq!(ranges[2].end_line, 7); // 2 lines
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_container_filtering_empty_function() {
        use crate::{File, FileMode, Section};
        use std::borrow::Cow;

        // Function with only unchanged sections - should be filtered out
        let source = r#"
fn unchanged_function() {
    println!("no changes here");
}
"#;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.rs")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: vec![Section::Unchanged {
                lines: vec![
                    Cow::Borrowed("fn unchanged_function() {\n"),
                    Cow::Borrowed("    println!(\"no changes here\");\n"),
                    Cow::Borrowed("}\n"),
                ],
            }],
            containers: None,
        };

        let result = try_add_semantic_containers(file, source, source);

        // Should have no containers since the function has no editable changes
        assert!(result.containers.is_none());
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_container_filtering_function_with_changes() {
        use crate::{ChangeType, File, FileMode, Section, SectionChangedLine};
        use std::borrow::Cow;

        // Function with changes - should be kept
        let old_source = r#"
fn modified_function() {
    println!("old");
}
"#;

        let new_source = r#"
fn modified_function() {
    println!("new");
}
"#;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.rs")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: vec![
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("\n"),
                        Cow::Borrowed("fn modified_function() {\n"),
                    ],
                },
                Section::Changed {
                    lines: vec![
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Removed,
                            line: Cow::Borrowed("    println!(\"old\");\n"),
                        },
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Added,
                            line: Cow::Borrowed("    println!(\"new\");\n"),
                        },
                    ],
                },
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("}\n"),
                    ],
                },
            ],
            containers: None,
        };

        let result = try_add_semantic_containers(file, old_source, new_source);

        // Should have containers with the function
        assert!(result.containers.is_some());
        let containers = result.containers.unwrap();
        assert_eq!(containers.len(), 1);

        match &containers[0] {
            crate::SemanticContainer::Function { name, section_indices, .. } => {
                assert_eq!(name, "modified_function");
                // Should have sections that fall within the function's line range
                // With context preservation, this includes both changed and unchanged sections
                assert!(!section_indices.is_empty());
                // At minimum, should have the Changed section
                assert!(section_indices.contains(&1));
            }
            _ => panic!("Expected Function container"),
        }
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_container_filtering_struct_with_no_field_changes() {
        use crate::{File, FileMode, Section};
        use std::borrow::Cow;

        // Struct where fields have no changes - should be filtered out
        let source = r#"
struct MyStruct {
    field1: i32,
    field2: String,
}
"#;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.rs")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: vec![Section::Unchanged {
                lines: vec![
                    Cow::Borrowed("struct MyStruct {\n"),
                    Cow::Borrowed("    field1: i32,\n"),
                    Cow::Borrowed("    field2: String,\n"),
                    Cow::Borrowed("}\n"),
                ],
            }],
            containers: None,
        };

        let result = try_add_semantic_containers(file, source, source);

        // Should have no containers since no fields have changes
        assert!(result.containers.is_none());
    }

    #[cfg(feature = "tree-sitter")]
    #[test]
    fn test_container_filtering_impl_with_mixed_methods() {
        use crate::{ChangeType, File, FileMode, Section, SectionChangedLine};
        use std::borrow::Cow;

        // Impl with one changed method and one unchanged - should keep only changed method
        let old_source = r#"
impl MyStruct {
    fn unchanged_method(&self) {
        println!("same");
    }

    fn changed_method(&self) {
        println!("old");
    }
}
"#;

        let new_source = r#"
impl MyStruct {
    fn unchanged_method(&self) {
        println!("same");
    }

    fn changed_method(&self) {
        println!("new");
    }
}
"#;

        let file = File {
            old_path: None,
            path: Cow::Borrowed(std::path::Path::new("test.rs")),
            file_mode: FileMode::FILE_DEFAULT,
            sections: vec![
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("\n"),
                        Cow::Borrowed("impl MyStruct {\n"),
                        Cow::Borrowed("    fn unchanged_method(&self) {\n"),
                        Cow::Borrowed("        println!(\"same\");\n"),
                        Cow::Borrowed("    }\n"),
                        Cow::Borrowed("\n"),
                        Cow::Borrowed("    fn changed_method(&self) {\n"),
                    ],
                },
                Section::Changed {
                    lines: vec![
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Removed,
                            line: Cow::Borrowed("        println!(\"old\");\n"),
                        },
                        SectionChangedLine {
                            is_checked: false,
                            change_type: ChangeType::Added,
                            line: Cow::Borrowed("        println!(\"new\");\n"),
                        },
                    ],
                },
                Section::Unchanged {
                    lines: vec![
                        Cow::Borrowed("    }\n"),
                        Cow::Borrowed("}\n"),
                    ],
                },
            ],
            containers: None,
        };

        let result = try_add_semantic_containers(file, old_source, new_source);

        // Should have containers with the impl
        assert!(result.containers.is_some());
        let containers = result.containers.unwrap();
        assert_eq!(containers.len(), 1);

        match &containers[0] {
            crate::SemanticContainer::Impl { type_name, methods, .. } => {
                assert_eq!(type_name, "MyStruct");
                // Should only have 1 method (the changed one)
                assert_eq!(methods.len(), 1);
                match &methods[0] {
                    crate::SemanticMember::Method { name, section_indices, .. } => {
                        assert_eq!(name, "changed_method");
                        // Should have sections within the method's line range
                        assert!(!section_indices.is_empty());
                        // At minimum, should have the Changed section
                        assert!(section_indices.contains(&1));
                    }
                    _ => panic!("Expected Method member"),
                }
            }
            _ => panic!("Expected Impl container"),
        }
    }
}
