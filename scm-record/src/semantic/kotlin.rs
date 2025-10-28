//! Kotlin semantic parsing.

use super::*;

/// Extract members (properties and methods) from a Kotlin class/object/interface.
#[cfg(feature = "tree-sitter")]
pub fn extract_members(
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
pub fn extract_containers_with_members(parsed: &ParsedFile) -> Vec<ContainerWithMembers> {
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
                        .map(|body| extract_members(body, source_bytes))
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
                        .map(|body| extract_members(body, source_bytes))
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
                        .map(|body| extract_members(body, source_bytes))
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

#[cfg(test)]
mod tests {
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // The function should start at line 1 (0-indexed) where the comment is
        assert_eq!(containers[0].container.start_line, 1);
        assert_eq!(containers[0].container.name, "commentedFunction");
    }
}
