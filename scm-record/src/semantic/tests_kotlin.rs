//! Tests for Kotlin semantic parsing.

use super::*;

#[test]
fn test_parser_creation_kotlin() {
    let result = create_parser(SupportedLanguage::Kotlin);
    assert!(result.is_ok());
}

#[test]
fn test_simple_kotlin_parse() {
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let source = "fun main() { println(\"Hello, world!\") }";
    let result = parse_source(&mut parser, source);
    assert!(result.is_ok());
}

#[test]
fn test_extract_kotlin_class() {
    let source = r#"
class Point(val x: Int, val y: Int) {
    fun distance(): Double {
        return sqrt(x * x + y * y)
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Point");
    assert!(matches!(containers[0].container.kind, ContainerKind::Class));
}

#[test]
fn test_extract_kotlin_class_with_methods() {
    let source = r#"
class Calculator {
    fun add(a: Int, b: Int): Int {
        return a + b
    }

    fun subtract(a: Int, b: Int): Int {
        return a - b
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
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
fn test_extract_kotlin_object() {
    let source = r#"
object Singleton {
    fun getInstance(): Singleton {
        return this
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Singleton");
    assert!(matches!(
        containers[0].container.kind,
        ContainerKind::Object
    ));
}

#[test]
fn test_extract_kotlin_interface() {
    let source = r#"
interface Drawable {
    fun draw()
    fun erase()
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "Drawable");
    assert!(matches!(
        containers[0].container.kind,
        ContainerKind::Interface
    ));
}

#[test]
fn test_extract_kotlin_top_level_function() {
    let source = r#"
fun calculateDistance(x1: Double, y1: Double, x2: Double, y2: Double): Double {
    val dx = x2 - x1
    val dy = y2 - y1
    return sqrt(dx * dx + dy * dy)
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].container.name, "calculateDistance");
    assert!(matches!(
        containers[0].container.kind,
        ContainerKind::Function
    ));
    assert_eq!(containers[0].members.len(), 0); // Functions have no members
}

#[test]
fn test_extract_kotlin_properties() {
    let source = r#"
class Person {
    val name: String = "John"
    var age: Int = 30

    fun birthday() {
        age++
    }
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    let container = &containers[0];
    assert_eq!(container.members.len(), 3); // 2 properties + 1 method

    // Properties
    assert_eq!(container.members[0].name, "name");
    assert!(matches!(container.members[0].kind, MemberKind::Property));

    assert_eq!(container.members[1].name, "age");
    assert!(matches!(container.members[1].kind, MemberKind::Property));

    // Method
    assert_eq!(container.members[2].name, "birthday");
    assert!(matches!(container.members[2].kind, MemberKind::Method));
}

#[test]
fn test_extract_kotlin_mixed_containers() {
    let source = r#"
class Point(val x: Int, val y: Int)

fun origin() = Point(0, 0)

object Constants {
    const val PI = 3.14159
}

interface Shape {
    fun area(): Double
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 4);

    assert_eq!(containers[0].container.name, "Point");
    assert!(matches!(containers[0].container.kind, ContainerKind::Class));

    assert_eq!(containers[1].container.name, "origin");
    assert!(matches!(
        containers[1].container.kind,
        ContainerKind::Function
    ));

    assert_eq!(containers[2].container.name, "Constants");
    assert!(matches!(
        containers[2].container.kind,
        ContainerKind::Object
    ));

    assert_eq!(containers[3].container.name, "Shape");
    assert!(matches!(
        containers[3].container.kind,
        ContainerKind::Interface
    ));
}

#[test]
fn test_kotlin_trivia_annotations() {
    let source = r#"
@Suppress("unused")
@Deprecated("Use new API")
class OldApi {
    fun oldMethod() {}
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The class should start at line 1 (0-indexed) where the first annotation is
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "OldApi");
}

#[test]
fn test_kotlin_trivia_comments() {
    let source = r#"
// This is a comment about the function
fun commentedFunction() {
    println("Hello")
}
"#;
    let mut parser = create_parser(SupportedLanguage::Kotlin).unwrap();
    let tree = parse_source(&mut parser, source).unwrap();
    let parsed = ParsedFile {
        source: source.to_string(),
        tree,
    };

    let containers = extract_kotlin_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The function should start at line 1 (0-indexed) where the comment is
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "commentedFunction");
}
