//! Python semantic parsing.

use super::*;

/// Extract methods from a Python class definition node.
#[cfg(feature = "tree-sitter")]
pub fn extract_methods(
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
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
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

                let methods = extract_methods(class_def, source_bytes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation_python() {
        let result = create_parser(SupportedLanguage::Python);
        assert!(result.is_ok());
    }

    #[test]
    fn test_simple_python_parse() {
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let source = "def hello():\n    print('Hello, world!')";
        let result = parse_source(&mut parser, source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_python_class() {
        let source = r#"
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].container.name, "Point");
        assert!(matches!(containers[0].container.kind, ContainerKind::Class));
    }

    #[test]
    fn test_extract_python_class_with_methods() {
        let source = r#"
class Calculator:
    def add(self, a, b):
        return a + b

    def subtract(self, a, b):
        return a - b
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.container.name, "Calculator");
        assert_eq!(container.members.len(), 2);

        assert_eq!(container.members[0].name, "add");
        assert!(matches!(container.members[0].kind, MemberKind::Method));

        assert_eq!(container.members[1].name, "subtract");
        assert!(matches!(container.members[1].kind, MemberKind::Method));
    }

    #[test]
    fn test_extract_python_top_level_function() {
        let source = r#"
def calculate_distance(p1, p2):
    dx = p2[0] - p1[0]
    dy = p2[1] - p1[1]
    return (dx * dx + dy * dy) ** 0.5
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].container.name, "calculate_distance");
        assert!(matches!(
            containers[0].container.kind,
            ContainerKind::Function
        ));
        assert_eq!(containers[0].members.len(), 0); // Functions have no members
    }

    #[test]
    fn test_extract_python_mixed_containers() {
        let source = r#"
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

def origin():
    return Point(0, 0)

class Vector:
    def magnitude(self):
        return 0
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 3);

        // First container: Point class
        assert_eq!(containers[0].container.name, "Point");
        assert!(matches!(containers[0].container.kind, ContainerKind::Class));
        assert_eq!(containers[0].members.len(), 1);
        assert_eq!(containers[0].members[0].name, "__init__");

        // Second container: origin function
        assert_eq!(containers[1].container.name, "origin");
        assert!(matches!(
            containers[1].container.kind,
            ContainerKind::Function
        ));

        // Third container: Vector class
        assert_eq!(containers[2].container.name, "Vector");
        assert!(matches!(containers[2].container.kind, ContainerKind::Class));
        assert_eq!(containers[2].members.len(), 1);
        assert_eq!(containers[2].members[0].name, "magnitude");
    }

    #[test]
    fn test_python_trivia_decorators() {
        let source = r#"
@dataclass
class Config:
    name: str
    value: int
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // The class should start at line 1 (0-indexed) where the decorator is
        assert_eq!(containers[0].container.start_line, 1);
        assert_eq!(containers[0].container.name, "Config");
    }

    #[test]
    fn test_python_trivia_method_decorators() {
        let source = r#"
class MyClass:
    @property
    def value(self):
        return self._value

    @staticmethod
    def helper():
        return 42
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.members.len(), 2);

        // Methods should include their decorators
        assert_eq!(container.members[0].name, "value");
        assert_eq!(container.members[1].name, "helper");
    }

    #[test]
    fn test_python_trivia_comments() {
        let source = r#"
# This is a comment about the function
def commented_function():
    pass
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // The function should start at line 1 (0-indexed) where the comment is
        assert_eq!(containers[0].container.start_line, 1);
        assert_eq!(containers[0].container.name, "commented_function");
    }

    #[test]
    fn test_python_trivia_combined_decorators_and_comments() {
        let source = r#"
# This class represents a configuration
@dataclass
@frozen
class Config:
    name: str
    value: int
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // Should start at line 1 where the comment is (before decorators)
        assert_eq!(containers[0].container.start_line, 1);
        assert_eq!(containers[0].container.name, "Config");
    }

    #[test]
    fn test_python_trivia_method_decorators_and_comments() {
        let source = r#"
class MyClass:
    # Property getter
    @property
    def value(self):
        return self._value

    # Static helper method
    # Does something useful
    @staticmethod
    def helper():
        return 42
"#;
        let mut parser = create_parser(SupportedLanguage::Python).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.members.len(), 2);

        // First method should include comment before decorator
        assert_eq!(container.members[0].name, "value");
        // Note: Currently starts at decorator line, not comment (trivia limitation)
        assert_eq!(container.members[0].start_line, 3); // Line of decorator

        // Second method should include both comments
        assert_eq!(container.members[1].name, "helper");
        // Note: Currently starts at first comment line
        assert_eq!(container.members[1].start_line, 7); // Line of first comment
    }
}
