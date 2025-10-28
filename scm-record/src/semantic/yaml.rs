//! YAML semantic parsing.

use super::*;

/// Extract containers with their members from a parsed YAML file.
/// Containers are top-level block mappings (key-value pairs).
#[cfg(feature = "tree-sitter")]
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
  let mut containers = Vec::new();
  let root_node = parsed.tree.root_node();
  let source_bytes = parsed.source.as_bytes();

  // The root is usually a stream_node containing document nodes
  let mut cursor = root_node.walk();
  for child in root_node.children(&mut cursor) {
    if child.kind() == "stream" || child.kind() == "document" {
      let mut doc_cursor = child.walk();
      for doc_child in child.children(&mut doc_cursor) {
        extract_yaml_mappings(doc_child, source_bytes, root_node, &mut containers);
      }
    } else if child.kind() == "block_mapping" || child.kind() == "block_sequence" {
      extract_yaml_mappings(child, source_bytes, root_node, &mut containers);
    }
  }

  containers
}

/// Helper to extract mappings from YAML nodes
#[cfg(feature = "tree-sitter")]
fn extract_yaml_mappings(
  node: tree_sitter::Node,
  source_bytes: &[u8],
  root_node: tree_sitter::Node,
  containers: &mut Vec<ContainerWithMembers>,
) {
  if node.kind() == "block_mapping" {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      if child.kind() == "block_mapping_pair" {
        // Get the key
        if let Some(key_node) = child.child_by_field_name("key") {
          let key_name = key_node
            .utf8_text(source_bytes)
            .unwrap_or("<unknown>")
            .trim()
            .to_string();

          let (start_line, end_line) =
            expand_range_for_trivia(child, node, &TriviaConfig::generic());

          containers.push(ContainerWithMembers {
            container: Container {
              kind: ContainerKind::Section { level: 1 }, // Use Section for YAML top-level keys
              name: key_name,
              start_line,
              end_line,
            },
            members: Vec::new(),
          });
        }
      }
    }
  } else if node.kind() == "block_node" || node.kind() == "document" {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
      extract_yaml_mappings(child, source_bytes, root_node, containers);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parser_creation_yaml() {
    let result = create_parser(SupportedLanguage::Yaml);
    assert!(result.is_ok());
  }

  #[test]
  fn test_simple_yaml_parse() {
    let mut parser = create_parser(SupportedLanguage::Yaml).unwrap();
    let source = "name: test\nvalue: 123";
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extract_yaml_top_level_keys() {
    let source = r#"name: myapp
version: 1.0.0
description: A test application
"#;
    let mut parser = create_parser(SupportedLanguage::Yaml).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);

    // Check that we found at least one of the expected keys
    let names: Vec<_> = containers
      .iter()
      .map(|c| c.container.name.as_str())
      .collect();
    assert!(
      names.contains(&"name") || names.contains(&"version") || names.contains(&"description")
    );
  }

  #[test]
  fn test_extract_yaml_nested_structure() {
    let source = r#"
database:
  host: localhost
  port: 5432

server:
  host: 0.0.0.0
  port: 8080
"#;
    let mut parser = create_parser(SupportedLanguage::Yaml).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);

    // Check for top-level keys
    let names: Vec<_> = containers
      .iter()
      .map(|c| c.container.name.as_str())
      .collect();
    assert!(names.contains(&"database") || names.contains(&"server"));
  }

  #[test]
  fn test_extract_yaml_with_list() {
    let source = r#"
dependencies:
  - express
  - react
  - webpack

devDependencies:
  - jest
  - eslint
"#;
    let mut parser = create_parser(SupportedLanguage::Yaml).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);

    let names: Vec<_> = containers
      .iter()
      .map(|c| c.container.name.as_str())
      .collect();
    assert!(names.contains(&"dependencies") || names.contains(&"devDependencies"));
  }

  #[test]
  fn test_yaml_with_comments() {
    let source = r#"
# Application configuration
app:
  name: myapp
  version: 1.0.0
"#;
    let mut parser = create_parser(SupportedLanguage::Yaml).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert!(containers.len() >= 1);

    // Find the app container
    let app_container = containers.iter().find(|c| c.container.name == "app");
    assert!(app_container.is_some());
  }
}
