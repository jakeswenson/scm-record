//! Example demonstrating tree-sitter semantic analysis.
//!
//! Run with: cargo run --example semantic_analysis --features tree-sitter

#[cfg(feature = "tree-sitter")]
fn main() {
    use scm_record::semantic::{parse_semantic_nodes, Language, SemanticNodeType};
    use std::path::Path;

    let rust_source = r#"
mod utils {
    pub fn helper() {
        println!("Helper function");
    }
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

fn main() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    println!("Distance: {}", p1.distance(&p2));
}

fn calculate_area(width: f64, height: f64) -> f64 {
    width * height
}
"#;

    println!("=== Tree-sitter Semantic Analysis Demo ===\n");

    // Test language detection
    let path = Path::new("example.rs");
    let language = Language::from_path(path);
    println!("Detected language for '{}': {:?}", path.display(), language);
    println!("Language supported: {}\n", language.is_supported());

    // Parse the source code
    println!("Parsing Rust source code...\n");
    match parse_semantic_nodes(language, rust_source) {
        Some(nodes) => {
            println!("Found {} semantic nodes:\n", nodes.len());
            for (i, node) in nodes.iter().enumerate() {
                let type_name = match &node.node_type {
                    SemanticNodeType::Function => "Function",
                    SemanticNodeType::Struct => "Struct",
                    SemanticNodeType::Impl => "Impl",
                    SemanticNodeType::Module => "Module",
                    SemanticNodeType::Block => "Block",
                    SemanticNodeType::Other(s) => s,
                };

                let name = node
                    .name
                    .as_ref()
                    .map(|n| format!(" '{}'", n))
                    .unwrap_or_default();

                println!(
                    "{}. {} {} (lines {}-{})",
                    i + 1,
                    type_name,
                    name,
                    node.start_line + 1, // Convert to 1-indexed for display
                    node.end_line + 1
                );
            }

            println!("\n=== Demonstration Complete ===");
            println!("\nThis shows that tree-sitter can identify semantic constructs");
            println!("in Rust code. The next step is to integrate this into the");
            println!("interactive UI for semantic-level change selection.");
        }
        None => {
            println!("Failed to parse source code");
        }
    }
}

#[cfg(not(feature = "tree-sitter"))]
fn main() {
    println!("This example requires the 'tree-sitter' feature.");
    println!("Run with: cargo run --example semantic_analysis --features tree-sitter");
}
