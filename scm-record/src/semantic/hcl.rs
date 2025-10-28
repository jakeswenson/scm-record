//! HCL (Terraform/OpenTofu) semantic parsing.

use super::*;

/// Extract containers with their members from a parsed HCL file.
#[cfg(feature = "tree-sitter")]
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
  let mut containers = Vec::new();
  let root_node = parsed.tree.root_node();
  let source_bytes = parsed.source.as_bytes();

  // HCL files have a root config_file node with a body child
  // We need to iterate through the body's children
  let mut cursor = root_node.walk();
  for child in root_node.children(&mut cursor) {
    if child.kind() == "body" {
      extract_hcl_blocks(child, source_bytes, root_node, &mut containers);
    }
  }

  containers
}

/// Helper to extract HCL blocks from a body node
#[cfg(feature = "tree-sitter")]
fn extract_hcl_blocks(
  body_node: tree_sitter::Node,
  source_bytes: &[u8],
  root_node: tree_sitter::Node,
  containers: &mut Vec<ContainerWithMembers>,
) {
  let mut cursor = body_node.walk();
  for child in body_node.children(&mut cursor) {
    match child.kind() {
      "block" => {
        // HCL blocks have children in order:
        // identifier "label1" "label2" { body }
        // Examples:
        // resource "aws_instance" "example" { ... }
        // variable "name" { ... }
        // output "name" { ... }

        let mut block_cursor = child.walk();
        let children: Vec<_> = child.children(&mut block_cursor).collect();

        // First child should be identifier (block type)
        let block_type = if !children.is_empty() && children[0].kind() == "identifier" {
          children[0]
            .utf8_text(source_bytes)
            .unwrap_or("<unknown>")
            .to_string()
        } else {
          continue;
        };

        // Collect all string_lit children as labels
        let labels: Vec<String> = children
          .iter()
          .filter(|n| n.kind() == "string_lit")
          .filter_map(|label| {
            label.utf8_text(source_bytes).ok().map(|s| {
              // Remove quotes from string literals
              s.trim_matches('"').to_string()
            })
          })
          .collect();

        let (kind, name) = match block_type.as_str() {
          "resource" => {
            if labels.len() >= 2 {
              (
                ContainerKind::Resource {
                  resource_type: labels[0].clone(),
                },
                labels[1].clone(),
              )
            } else {
              continue;
            }
          }
          "data" => {
            if labels.len() >= 2 {
              (
                ContainerKind::DataSource {
                  data_type: labels[0].clone(),
                },
                labels[1].clone(),
              )
            } else {
              continue;
            }
          }
          "variable" => {
            if let Some(name) = labels.first() {
              (ContainerKind::Variable, name.clone())
            } else {
              continue;
            }
          }
          "output" => {
            if let Some(name) = labels.first() {
              (ContainerKind::Output, name.clone())
            } else {
              continue;
            }
          }
          "module" => {
            if let Some(name) = labels.first() {
              (ContainerKind::Module, name.clone())
            } else {
              continue;
            }
          }
          _ => continue, // Skip other block types (locals, terraform, etc.)
        };

        let (start_line, end_line) =
          expand_range_for_trivia(child, root_node, &TriviaConfig::hcl());

        containers.push(ContainerWithMembers {
          container: Container {
            kind,
            name,
            start_line,
            end_line,
          },
          members: Vec::new(), // HCL blocks don't have members in our model
        });
      }
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parser_creation_hcl() {
    let result = create_parser(SupportedLanguage::Hcl);
    assert!(result.is_ok());
  }

  #[test]
  fn test_hcl_tree_structure_debug() {
    let source = r#"variable "name" { default = "test" }"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();

    eprintln!("\n=== HCL Tree Structure ===");
    eprintln!("Root kind: {}", tree.root_node().kind());

    let mut cursor = tree.root_node().walk();
    for child in tree.root_node().children(&mut cursor) {
      eprintln!("Child kind: {}", child.kind());

      if child.kind() == "body" {
        let mut body_cursor = child.walk();
        for block in child.children(&mut body_cursor) {
          eprintln!("  Block kind: {}", block.kind());

          if let Some(type_field) = block.child_by_field_name("type") {
            eprintln!(
              "    Has 'type' field: {:?}",
              type_field.utf8_text(source.as_bytes())
            );
          }

          // List all children
          let mut block_cursor = block.walk();
          for block_child in block.children(&mut block_cursor) {
            eprintln!("    Block child kind: {}", block_child.kind());
          }
        }
      }
    }
  }

  #[test]
  fn test_simple_hcl_parse() {
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let source = r#"variable "name" { default = "test" }"#;
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extract_hcl_resource() {
    let source = r#"
resource "aws_instance" "example" {
    ami           = "ami-12345678"
    instance_type = "t2.micro"
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "example");
    if let ContainerKind::Resource { resource_type } = &containers[0].container.kind {
      assert_eq!(resource_type, "aws_instance");
    } else {
      panic!("Expected Resource container");
    }
  }

  #[test]
  fn test_extract_hcl_data_source() {
    let source = r#"
data "aws_ami" "ubuntu" {
    most_recent = true
    owners      = ["099720109477"]
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "ubuntu");
    if let ContainerKind::DataSource { data_type } = &containers[0].container.kind {
      assert_eq!(data_type, "aws_ami");
    } else {
      panic!("Expected DataSource container");
    }
  }

  #[test]
  fn test_extract_hcl_variable() {
    let source = r#"
variable "instance_count" {
    description = "Number of instances"
    type        = number
    default     = 1
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "instance_count");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Variable
    ));
  }

  #[test]
  fn test_extract_hcl_output() {
    let source = r#"
output "instance_ip" {
    description = "The public IP address of the instance"
    value       = aws_instance.example.public_ip
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "instance_ip");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Output
    ));
  }

  #[test]
  fn test_extract_hcl_module() {
    let source = r#"
module "vpc" {
    source = "./modules/vpc"
    cidr   = "10.0.0.0/16"
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "vpc");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Module
    ));
  }

  #[test]
  fn test_extract_hcl_mixed_containers() {
    let source = r#"
variable "region" {
    default = "us-west-2"
}

resource "aws_instance" "web" {
    ami           = "ami-12345678"
    instance_type = "t2.micro"
}

output "public_ip" {
    value = aws_instance.web.public_ip
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 3);

    assert_eq!(containers[0].container.name, "region");
    assert!(matches!(
      containers[0].container.kind,
      ContainerKind::Variable
    ));

    assert_eq!(containers[1].container.name, "web");
    assert!(matches!(
      containers[1].container.kind,
      ContainerKind::Resource { .. }
    ));

    assert_eq!(containers[2].container.name, "public_ip");
    assert!(matches!(
      containers[2].container.kind,
      ContainerKind::Output
    ));
  }

  #[test]
  fn test_hcl_trivia_comments() {
    let source = r#"
# This is a variable for the region
variable "region" {
    default = "us-west-2"
}
"#;
    let mut parser = create_parser(SupportedLanguage::Hcl).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
      source: source.to_string(),
      tree,
    };

    let containers = extract_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // Note: Currently starts at variable block, not comment (trivia limitation)
    // TODO: Fix trivia handling to include comments before blocks
    assert_eq!(containers[0].container.start_line, 2);
    assert_eq!(containers[0].container.name, "region");
  }
}
