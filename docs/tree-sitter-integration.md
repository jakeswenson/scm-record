# Tree-sitter Integration

This document describes the tree-sitter integration for semantic-level change selection in scm-record.

## Overview

Tree-sitter integration enables selecting changes at semantic boundaries (functions, classes, methods, etc.) rather than just at the line level. This makes it easier to commit logically-related changes together.

## Status

**Phase 1: Foundation (In Progress)**

- ✅ Tree-sitter dependencies added
- ✅ Semantic module created
- ✅ Language detection implemented
- ✅ Rust parsing implemented
- ⏳ Data model integration (pending)
- ⏳ UI integration (pending)

## Building with Tree-sitter Support

The tree-sitter feature is optional and opt-in:

```bash
# Build with tree-sitter support
cargo build --features tree-sitter

# Run tests with tree-sitter support
cargo test --features tree-sitter

# Run the semantic analysis example
cargo run --example semantic_analysis --features tree-sitter
```

## Supported Languages

### Currently Implemented
- **Rust** - Functions, structs, impl blocks, modules

### Planned (Priority Order)
1. OpenTofu/Terraform
2. Markdown
3. TypeScript & JavaScript
4. HTML
5. Svelte
6. Python
7. Zig
8. Swift
9. Java
10. Lean4
11. Haskell
12. Kotlin
13. Scala
14. TOML
15. YAML
16. JSON

## API Usage

```rust
use scm_record::semantic::{Language, parse_semantic_nodes};
use std::path::Path;

// Detect language from file path
let language = Language::from_path(Path::new("example.rs"));

// Parse semantic nodes
let source = "fn hello() { println!(\"Hello\"); }";
if let Some(nodes) = parse_semantic_nodes(language, source) {
    for node in nodes {
        println!("{:?}: {} (lines {}-{})",
            node.node_type,
            node.name.unwrap_or_default(),
            node.start_line,
            node.end_line
        );
    }
}
```

## Architecture

```
┌─────────────────────────────────────────┐
│           scm-record UI                 │
│     (Interactive change selector)       │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│      Semantic Module (semantic.rs)      │
│  - Language detection                   │
│  - Parser management                    │
│  - Semantic node extraction             │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          tree-sitter Library            │
│  - Language grammars (tree-sitter-rust) │
│  - Parsing engine                       │
│  - Query system                         │
└─────────────────────────────────────────┘
```

## Semantic Node Types

The following semantic constructs are recognized:

- **Function** - Function and method definitions
- **Struct** - Struct, class, or type definitions
- **Impl** - Implementation blocks
- **Module** - Module definitions
- **Block** - Code blocks (if, for, while, etc.)
- **Other** - Other language-specific constructs

## Data Structures

### SemanticNode

```rust
pub struct SemanticNode {
    /// The type of this semantic node
    pub node_type: SemanticNodeType,
    /// The name of this node (e.g., function name), if available
    pub name: Option<String>,
    /// The starting line (0-indexed)
    pub start_line: usize,
    /// The ending line (0-indexed, inclusive)
    pub end_line: usize,
    /// Child semantic nodes
    pub children: Vec<SemanticNode>,
}
```

## Implementation Phases

See [ADR 001](adrs/001-tree-sitter-integration.md) for the complete implementation plan.

### Phase 1: Foundation ✅
- Tree-sitter dependencies
- Semantic module
- Language detection
- Basic Rust parsing

### Phase 2: Data Model (Next)
- Extend Section enum
- Track semantic parents
- Handle partial selections

### Phase 3: UI Integration
- Keyboard navigation
- Visual indicators
- Semantic node rendering

### Phase 4: Polish
- Performance optimization
- Error handling
- Configuration options

### Phase 5: Expand Languages
- Add remaining priority languages
- Test across all supported languages

## Testing

Run the semantic analysis example to see tree-sitter in action:

```bash
cargo run --example semantic_analysis --features tree-sitter
```

Expected output:
```
=== Tree-sitter Semantic Analysis Demo ===

Detected language for 'example.rs': Rust
Language supported: true

Parsing Rust source code...

Found 6 semantic nodes:

1. Module 'utils' (lines 2-6)
2. Function 'helper' (lines 3-5)
3. Struct 'Point' (lines 8-11)
4. Impl 'Point' (lines 13-25)
5. Function 'main' (lines 27-31)
6. Function 'calculate_area' (lines 33-35)
```

## Contributing

To add support for a new language:

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add the language to the `Language` enum in `semantic.rs`
3. Implement a `parse_<language>` function
4. Add the language to `parse_semantic_nodes` match statement
5. Write tests for the new language
6. Update this documentation

## Future Enhancements

- Nested structure support (methods within impl blocks)
- Change attribution across refactorings
- Caching parsed trees for performance
- Custom query configuration
- Integration with diffsitter for semantic diffs

## References

- [ADR 001: Tree-sitter Integration](adrs/001-tree-sitter-integration.md)
- [tree-sitter](https://tree-sitter.github.io/)
- [Issue #2: Integrate tree-sitter into this tool](https://github.com/jakeswenson/scm-record/issues/2)
