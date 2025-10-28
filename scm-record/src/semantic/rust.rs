//! Rust semantic parsing.

use super::*;

/// Extract Rust containers from a parsed syntax tree.
#[cfg(feature = "tree-sitter")]
pub fn extract_containers(parsed: &ParsedFile) -> Vec<Container> {
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

          let trait_name =
            trait_node.and_then(|node| node.utf8_text(source_bytes).ok().map(|s| s.to_string()));

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

/// Extract containers with their members from a parsed Rust file.
///
/// Returns a vector of containers (structs, impls, functions) with their associated
/// members (fields, methods). Line ranges are expanded to include attributes and comments.
#[cfg(feature = "tree-sitter")]
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
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

          let trait_name =
            trait_node.and_then(|node| node.utf8_text(source_bytes).ok().map(|s| s.to_string()));

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
      "mod_item" => {
        // Extract the module itself
        if let Some(name_node) = child.child_by_field_name("name") {
          let module_name = name_node
            .utf8_text(source_bytes)
            .unwrap_or("<unknown>")
            .to_string();

          let (start_line, end_line) = expand_range_for_attributes_and_comments(child, root_node);

          containers.push(ContainerWithMembers {
            container: Container {
              kind: ContainerKind::Module,
              name: module_name,
              start_line,
              end_line,
            },
            members: Vec::new(),
          });
        }

        // Also extract functions inside the module as separate containers
        // This is important for test modules where each test function should be navigable
        if let Some(body) = child.child_by_field_name("body") {
          let mut body_cursor = body.walk();
          for item in body.children(&mut body_cursor) {
            if item.kind() == "function_item" {
              if let Some(name_node) = item.child_by_field_name("name") {
                let name = name_node
                  .utf8_text(source_bytes)
                  .unwrap_or("<unknown>")
                  .to_string();

                let (start_line, end_line) = expand_range_for_attributes_and_comments(item, body);

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
          }
        }
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
  fn test_parser_creation_rust() {
    let result = create_parser(SupportedLanguage::Rust);
    assert!(result.is_ok());
  }

  #[test]
  fn test_simple_rust_parse() {
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let source = "fn main() { println!(\"Hello, world!\"); }";
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
  }

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

    let containers = extract_containers(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "Point");
    assert!(matches!(containers[0].kind, ContainerKind::Struct));
  }

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

    let containers = extract_containers(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "Point");
    assert!(matches!(
      containers[0].kind,
      ContainerKind::Impl { trait_name: None }
    ));
  }

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

    let containers = extract_containers(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "Point");
    if let ContainerKind::Impl { trait_name } = &containers[0].kind {
      assert_eq!(trait_name.as_deref(), Some("Display"));
    } else {
      panic!("Expected Impl with trait");
    }
  }

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

    let containers = extract_containers(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "calculate_distance");
    assert!(matches!(containers[0].kind, ContainerKind::Function));
  }

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

    let containers = extract_containers(&parsed);
    assert_eq!(containers.len(), 3);

    assert_eq!(containers[0].name, "Point");
    assert!(matches!(containers[0].kind, ContainerKind::Struct));

    assert_eq!(containers[1].name, "Point");
    assert!(matches!(
      containers[1].kind,
      ContainerKind::Impl { trait_name: None }
    ));

    assert_eq!(containers[2].name, "origin");
    assert!(matches!(containers[2].kind, ContainerKind::Function));
  }

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

    let old_containers = extract_containers(&old_parsed);
    let new_containers = extract_containers(&new_parsed);

    assert_eq!(old_containers.len(), 1);
    assert_eq!(new_containers.len(), 1);
    assert_eq!(old_containers[0].name, "Point");
    assert_eq!(new_containers[0].name, "Point");
  }

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

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    let container = &containers[0];
    assert_eq!(container.container.name, "Point");
    assert_eq!(container.members.len(), 2);

    assert_eq!(container.members[0].name, "x");
    assert!(matches!(container.members[0].kind, MemberKind::Field));

    assert_eq!(container.members[1].name, "y");
    assert!(matches!(container.members[1].kind, MemberKind::Field));
  }

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

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    let container = &containers[0];
    assert_eq!(container.container.name, "Point");
    assert_eq!(container.members.len(), 2);

    assert_eq!(container.members[0].name, "new");
    assert!(matches!(container.members[0].kind, MemberKind::Method));

    assert_eq!(container.members[1].name, "distance");
    assert!(matches!(container.members[1].kind, MemberKind::Method));
  }

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

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 2);

    // Struct with fields
    assert_eq!(containers[0].container.name, "Point");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Struct
    ));
    assert_eq!(containers[0].members.len(), 2);
    assert_eq!(containers[0].members[0].name, "x");
    assert_eq!(containers[0].members[1].name, "y");

    // Impl with methods
    assert_eq!(containers[1].container.name, "Point");
    assert!(matches!(
      containers[1].container.kind,
      ContainerKind::Impl { trait_name: None }
    ));
    assert_eq!(containers[1].members.len(), 2);
    assert_eq!(containers[1].members[0].name, "new");
    assert_eq!(containers[1].members[1].name, "origin");
  }

  #[test]
  fn test_rust_trivia_attributes() {
    let source = r#"
#[derive(Debug)]
#[cfg(test)]
struct TestStruct {
    field: i32,
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The struct should start at line 1 (0-indexed) where the first attribute is
    assert_eq!(containers[0].container.start_line, 1);
  }

  #[test]
  fn test_rust_trivia_doc_comments() {
    let source = r#"
/// This is a doc comment
/// for the function
fn documented_function() {
    println!("Hello");
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The function should start at line 1 (0-indexed) where the doc comment starts
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "documented_function");
  }

  #[test]
  fn test_rust_trivia_combined_attributes_and_doc_comments() {
    let source = r#"
/// Documentation for the struct
/// with multiple lines
#[derive(Debug, Clone)]
#[cfg(test)]
struct TestStruct {
    field: i32,
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // Should start at line 1 where the doc comment starts
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "TestStruct");
  }

  #[test]
  fn test_rust_trivia_method_with_attributes_and_comments() {
    let source = r#"
impl Point {
    /// Gets the X coordinate
    #[inline]
    fn get_x(&self) -> i32 {
        self.x
    }

    /// Gets the Y coordinate
    #[inline]
    #[must_use]
    fn get_y(&self) -> i32 {
        self.y
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].members.len(), 2);

    // First method should include its doc comment and attribute
    assert_eq!(containers[0].members[0].name, "get_x");
    assert_eq!(containers[0].members[0].start_line, 2); // Line of doc comment

    // Second method should include both doc comment and both attributes
    assert_eq!(containers[0].members[1].name, "get_y");
    assert_eq!(containers[0].members[1].start_line, 8); // Line of doc comment
  }

  #[test]
  fn test_rust_whitespace_attribute_function() {
    let source = r#"

#[test]
fn test_something() {
    assert!(true);
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1, "Should find the function");
    assert_eq!(containers[0].container.name, "test_something");
    // The function should include the attribute in its range
    assert_eq!(containers[0].container.start_line, 2); // Line with #[test]
  }

  #[test]
  fn test_rust_multiple_blanks_attribute_function() {
    let source = r#"



#[cfg(test)]
fn another_test() {
    assert!(true);
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(
      containers.len(),
      1,
      "Should find the function even after multiple blank lines"
    );
    assert_eq!(containers[0].container.name, "another_test");
    assert_eq!(containers[0].container.start_line, 4); // Line with #[cfg(test)]
  }

  #[test]
  fn test_module_with_multiple_test_functions() {
    // This reproduces the issue where test functions inside a module
    // should be recognized as separate containers, not just as members of the module
    let source = r#"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!(true);
    }

    #[test]
    fn test_two() {
        assert!(true);
    }

    #[test]
    fn test_three() {
        assert!(true);
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Rust).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);

    // Now we extract both the module AND the functions inside it as separate containers
    // This allows each test function to be navigable independently in the diff view
    assert_eq!(containers.len(), 4, "Should find module + 3 test functions");

    // First container should be the module
    assert_eq!(containers[0].container.name, "tests");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Module
    ));

    // Next three containers should be the test functions
    assert_eq!(containers[1].container.name, "test_one");
    assert!(matches!(
      containers[1].container.kind,
      ContainerKind::Function
    ));

    assert_eq!(containers[2].container.name, "test_two");
    assert!(matches!(
      containers[2].container.kind,
      ContainerKind::Function
    ));

    assert_eq!(containers[3].container.name, "test_three");
    assert!(matches!(
      containers[3].container.kind,
      ContainerKind::Function
    ));
  }
}
