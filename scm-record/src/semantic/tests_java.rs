//! Tests for Java semantic parsing.

use super::*;

#[test]
fn test_parser_creation_java() {
    let result = create_parser(SupportedLanguage::Java);
    assert!(result.is_ok());
}

#[test]
fn test_simple_java_parse() {
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let source = "class Hello { public static void main(String[] args) { System.out.println(\"Hello\"); } }";
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
}

#[test]
fn test_extract_java_class() {
    let source = r#"
class Point {
    private int x;
    private int y;

    public Point(int x, int y) {
        this.x = x;
        this.y = y;
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Point");
    assert!(matches!(containers[0].container.kind, ContainerKind::Class));
}

#[test]
fn test_extract_java_class_with_methods() {
    let source = r#"
class Calculator {
    public int add(int a, int b) {
        return a + b;
    }

    public int subtract(int a, int b) {
        return a - b;
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
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
fn test_extract_java_interface() {
    let source = r#"
interface Drawable {
    void draw();
    void erase();
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Drawable");
    assert!(matches!(
        containers[0].container.kind,
        ContainerKind::Interface
    ));
    assert_eq!(containers[0].members.len(), 2);
}

#[test]
fn test_extract_java_enum() {
    let source = "
enum Color {
    RED, GREEN, BLUE;

    public String getHex() {
        return \"#000000\";
    }
}
";
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Color");
    assert!(matches!(containers[0].container.kind, ContainerKind::Enum));
}

#[test]
fn test_extract_java_fields() {
    let source = r#"
class Person {
    private String name;
    private int age;

    public void birthday() {
        age++;
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    let container = &containers[0];
    assert_eq!(container.members.len(), 3); // 2 fields + 1 method

    // Fields
    assert_eq!(container.members[0].name, "name");
    assert!(matches!(container.members[0].kind, MemberKind::Field));

    assert_eq!(container.members[1].name, "age");
    assert!(matches!(container.members[1].kind, MemberKind::Field));

    // Method
    assert_eq!(container.members[2].name, "birthday");
    assert!(matches!(container.members[2].kind, MemberKind::Method));
}

#[test]
fn test_extract_java_constructor() {
    let source = r#"
class Point {
    private int x, y;

    public Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    public int getX() {
        return x;
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    let container = &containers[0];
    // 1 field (x, even though there's also y), 1 constructor, 1 method
    assert!(container.members.len() >= 2);

    // Constructor should be extracted
    assert!(container
        .members
        .iter()
        .any(|m| m.name == "Point" && matches!(m.kind, MemberKind::Method)));

    // Method should be extracted
    assert!(container
        .members
        .iter()
        .any(|m| m.name == "getX" && matches!(m.kind, MemberKind::Method)));
}

#[test]
fn test_extract_java_mixed_containers() {
    let source = r#"
class Point {
    int x, y;
}

interface Shape {
    double area();
}

enum Color {
    RED, GREEN, BLUE
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 3);

    assert_eq!(containers[0].container.name, "Point");
    assert!(matches!(containers[0].container.kind, ContainerKind::Class));

    assert_eq!(containers[1].container.name, "Shape");
    assert!(matches!(
        containers[1].container.kind,
        ContainerKind::Interface
    ));

    assert_eq!(containers[2].container.name, "Color");
    assert!(matches!(containers[2].container.kind, ContainerKind::Enum));
}

#[test]
fn test_java_trivia_annotations() {
    let source = r#"
@Deprecated
@SuppressWarnings("unchecked")
class OldApi {
    @Override
    public String toString() {
        return "OldApi";
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The class should start at line 1 (0-indexed) where the first annotation is
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "OldApi");
}

#[test]
fn test_java_trivia_javadoc() {
    let source = r#"
/**
 * This is a Javadoc comment
 * for the class
 */
class DocumentedClass {
    /**
     * This method does something
     */
    public void doSomething() {}
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The class should start at line 1 (0-indexed) where the javadoc starts
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "DocumentedClass");
}

#[test]
fn test_java_trivia_line_comments() {
    let source = r#"
// This is a comment about the class
class CommentedClass {
    public void method() {}
}
"#;
    let mut parser = create_parser(SupportedLanguage::Java).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_java_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The class should start at line 1 (0-indexed) where the comment is
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "CommentedClass");
}
