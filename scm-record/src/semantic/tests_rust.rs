//! Tests for Rust semantic parsing.

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

    let containers = extract_rust_containers(&parsed);
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

    let containers = extract_rust_containers(&parsed);
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

    let containers = extract_rust_containers(&parsed);
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

    let containers = extract_rust_containers(&parsed);
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

    let containers = extract_rust_containers(&parsed);
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

    let old_containers = extract_rust_containers(&old_parsed);
    let new_containers = extract_rust_containers(&new_parsed);

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

    let containers = extract_rust_containers_with_members(&parsed);
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

    let containers = extract_rust_containers_with_members(&parsed);
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

    let containers = extract_rust_containers_with_members(&parsed);
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

    let containers = extract_rust_containers_with_members(&parsed);
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

    let containers = extract_rust_containers_with_members(&parsed);
    assert_eq!(containers.len(), 1);

    // The function should start at line 1 (0-indexed) where the doc comment starts
    assert_eq!(containers[0].container.start_line, 1);
    assert_eq!(containers[0].container.name, "documented_function");
}
