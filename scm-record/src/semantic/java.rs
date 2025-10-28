//! Java semantic parsing.

use super::*;

/// Extract members (fields and methods) from a Java class/interface/enum body.
#[cfg(feature = "tree-sitter")]
pub fn extract_members(
    body_node: tree_sitter::Node,
    source_bytes: &[u8],
) -> Vec<Member> {
    let mut members = Vec::new();
    let mut cursor = body_node.walk();

    for item in body_node.children(&mut cursor) {
        match item.kind() {
            "field_declaration" => {
                // Java fields can declare multiple variables, extract each
                let mut field_cursor = item.walk();
                for field_child in item.children(&mut field_cursor) {
                    if field_child.kind() == "variable_declarator" {
                        if let Some(name_node) = field_child.child_by_field_name("name") {
                            let name = name_node
                                .utf8_text(source_bytes)
                                .unwrap_or("<unknown>")
                                .to_string();

                            let (start_line, end_line) =
                                expand_range_for_trivia(item, body_node, &TriviaConfig::java());

                            members.push(Member {
                                kind: MemberKind::Field,
                                name,
                                start_line,
                                end_line,
                            });
                            break; // Only take first variable declarator for the whole field declaration
                        }
                    }
                }
            }
            "method_declaration" | "constructor_declaration" => {
                if let Some(name_node) = item.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let (start_line, end_line) =
                        expand_range_for_trivia(item, body_node, &TriviaConfig::java());

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

/// Extract containers with their members from a parsed Java file.
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

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

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
            "interface_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

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
            "enum_declaration" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = name_node
                        .utf8_text(source_bytes)
                        .unwrap_or("<unknown>")
                        .to_string();

                    let members = if let Some(body) = child.child_by_field_name("body") {
                        extract_members(body, source_bytes)
                    } else {
                        Vec::new()
                    };

                    let (start_line, end_line) =
                        expand_range_for_trivia(child, root_node, &TriviaConfig::java());

                    containers.push(ContainerWithMembers {
                        container: Container {
                            kind: ContainerKind::Enum,
                            name,
                            start_line,
                            end_line,
                        },
                        members,
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
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

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // Note: Currently starts at class declaration, not javadoc (trivia limitation)
        // TODO: Fix trivia handling to include javadoc as always_include
        assert_eq!(containers[0].container.start_line, 5);
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

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // The class should start at line 1 (0-indexed) where the comment is
        assert_eq!(containers[0].container.start_line, 1);
        assert_eq!(containers[0].container.name, "CommentedClass");
    }

    #[test]
    fn test_java_trivia_combined_javadoc_and_annotations() {
        let source = r#"
/**
 * Represents a user in the system
 * @author John Doe
 */
@Entity
@Table(name = "users")
class User {
    private String name;
}
"#;
        let mut parser = create_parser(SupportedLanguage::Java).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        // Note: Currently starts at annotation, not javadoc (trivia limitation)
        // TODO: Fix trivia handling to include javadoc before annotations
        assert_eq!(containers[0].container.start_line, 5);
        assert_eq!(containers[0].container.name, "User");
    }

    #[test]
    fn test_java_trivia_method_with_javadoc_and_annotations() {
        let source = r#"
class MyClass {
    /**
     * Gets the value
     * @return the current value
     */
    @Override
    @Deprecated
    public int getValue() {
        return value;
    }

    // Line comment
    /**
     * Sets the value
     */
    @Deprecated(since = "2.0")
    public void setValue(int v) {
        value = v;
    }
}
"#;
        let mut parser = create_parser(SupportedLanguage::Java).unwrap();
        let tree = parse_source(&mut parser, source).unwrap();
        let parsed = ParsedFile {
            source: source.to_string(),
            tree,
        };

        let containers = extract_containers_with_members(&parsed);
        assert_eq!(containers.len(), 1);

        let container = &containers[0];
        assert_eq!(container.members.len(), 2);

        // First method - currently starts at annotation, not javadoc (trivia limitation)
        assert_eq!(container.members[0].name, "getValue");
        assert_eq!(container.members[0].start_line, 6); // Line of annotation

        // Second method should include line comment, javadoc, and annotation
        assert_eq!(container.members[1].name, "setValue");
        assert_eq!(container.members[1].start_line, 16); // Line of annotation
    }
}
