//! Markdown semantic parsing.

use super::*;

/// Helper to extract headings from a section node (recursively handles nested sections)
#[cfg(feature = "tree-sitter")]
fn extract_headings_from_section(
  section_node: tree_sitter::Node,
  source_bytes: &[u8],
  root_node: tree_sitter::Node,
  containers: &mut Vec<ContainerWithMembers>,
) {
  let mut cursor = section_node.walk();
  for child in section_node.children(&mut cursor) {
    match child.kind() {
      "section" => {
        // Recursively extract from nested sections
        extract_headings_from_section(child, source_bytes, root_node, containers);
      }
      "atx_heading" | "setext_heading" => {
        // Determine heading level
        let level = if child.kind() == "atx_heading" {
          let mut found_level = 1;
          let mut level_cursor = child.walk();
          for marker_child in child.children(&mut level_cursor) {
            match marker_child.kind() {
              "atx_h1_marker" => {
                found_level = 1;
                break;
              }
              "atx_h2_marker" => {
                found_level = 2;
                break;
              }
              "atx_h3_marker" => {
                found_level = 3;
                break;
              }
              "atx_h4_marker" => {
                found_level = 4;
                break;
              }
              "atx_h5_marker" => {
                found_level = 5;
                break;
              }
              "atx_h6_marker" => {
                found_level = 6;
                break;
              }
              _ => continue,
            }
          }
          found_level
        } else {
          // setext_heading: Level 1 uses =, Level 2 uses -
          let text = child.utf8_text(source_bytes).unwrap_or("");
          if text.contains("====") || text.contains("===") {
            1
          } else {
            2
          }
        };

        // Extract heading text
        let heading_text = {
          let mut text_cursor = child.walk();
          // ATX headings have "inline" nodes, setext headings have "paragraph" nodes
          let content_node = child
            .children(&mut text_cursor)
            .find(|n| n.kind() == "inline" || n.kind() == "paragraph");

          content_node
            .and_then(|n| n.utf8_text(source_bytes).ok())
            .unwrap_or("<unknown>")
            .trim()
            .to_string()
        };

        let (start_line, end_line) =
          expand_range_for_trivia(child, root_node, &TriviaConfig::generic());

        containers.push(ContainerWithMembers {
          container: Container {
            kind: ContainerKind::Section { level },
            name: heading_text,
            start_line,
            end_line,
          },
          members: Vec::new(), // Markdown sections don't have members
        });
      }
      _ => {}
    }
  }
}

/// Extract containers with their members from a parsed Markdown file.
/// Containers are sections based on headers (# Header, ## Subheader, etc.)
#[cfg(feature = "tree-sitter")]
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
  let mut containers = Vec::new();
  let root_node = parsed.tree.root_node();
  let source_bytes = parsed.source.as_bytes();

  let mut cursor = root_node.walk();
  for child in root_node.children(&mut cursor) {
    match child.kind() {
      "section" => {
        // Markdown wraps headings in section nodes
        // Extract headings from within sections
        extract_headings_from_section(child, source_bytes, root_node, &mut containers);
      }
      _ => {}
    }
  }

  containers
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parser_creation_markdown() {
    let result = create_parser(SupportedLanguage::Markdown);
    assert!(result.is_ok());
  }

  #[test]
  fn test_simple_markdown_parse() {
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let source = "# Hello World\n\nSome content.";
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extract_markdown_atx_heading() {
    let source = r#"# Main Header

Some content here.
"#;
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);
    assert_eq!(containers[0].container.name, "Main Header");
    if let ContainerKind::Section { level } = containers[0].container.kind {
      assert_eq!(level, 1);
    } else {
      panic!("Expected Section container");
    }
  }

  #[test]
  fn test_extract_markdown_multiple_levels() {
    let source = r#"# Level 1

## Level 2

### Level 3

Some content.
"#;
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 3);

    assert_eq!(containers[0].container.name, "Level 1");
    if let ContainerKind::Section { level } = containers[0].container.kind {
      assert_eq!(level, 1);
    } else {
      panic!("Expected Section container");
    }

    assert_eq!(containers[1].container.name, "Level 2");
    if let ContainerKind::Section { level } = containers[1].container.kind {
      assert_eq!(level, 2);
    } else {
      panic!("Expected Section container");
    }

    assert_eq!(containers[2].container.name, "Level 3");
    if let ContainerKind::Section { level } = containers[2].container.kind {
      assert_eq!(level, 3);
    } else {
      panic!("Expected Section container");
    }
  }

  #[test]
  fn test_extract_markdown_setext_heading() {
    let source = "Main Header
===========

Some content.
";
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);
    assert_eq!(containers[0].container.name, "Main Header");
    if let ContainerKind::Section { level } = containers[0].container.kind {
      assert_eq!(level, 1);
    } else {
      panic!("Expected Section container");
    }
  }

  #[test]
  fn test_extract_markdown_mixed_headings() {
    let source = r#"# Introduction

Some intro content.

## Features

Feature list here.

### Installation

Installation steps.
"#;
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 3);

    assert_eq!(containers[0].container.name, "Introduction");
    assert_eq!(containers[1].container.name, "Features");
    assert_eq!(containers[2].container.name, "Installation");
  }

  #[test]
  fn test_markdown_sections_have_no_members() {
    // Regression test: ensure markdown sections have empty members list
    // so they get section assignments via the "no members" path in
    // enhance_file_with_containers, not the "assign to members" path.
    let source = r#"# Main Header

Some content.

## Subheader

More content.
"#;
    let mut parser = create_parser(SupportedLanguage::Markdown).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 2);

    // All markdown sections should have no members
    for container_with_members in &containers {
      assert!(
        container_with_members.members.is_empty(),
        "Markdown sections should not have members, found {} members for '{}'",
        container_with_members.members.len(),
        container_with_members.container.name
      );
    }
  }
}
