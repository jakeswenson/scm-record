//! Tests for Python semantic parsing.

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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
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

    let containers = extract_python_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The function should start at line 1 (0-indexed) where the comment is
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "commented_function");
}
