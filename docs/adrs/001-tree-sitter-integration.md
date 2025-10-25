# ADR 001: Tree-sitter Integration for Semantic-Level Change Selection

## Status

Proposed

## Context

### Current State

**scm-record** currently uses a 3-level hierarchy for selection:
1. **File** level (e.g., `foo/bar.rs`)
2. **Section** level (contiguous blocks of changes)
3. **Line** level (individual added/removed lines)

Users navigate with `hjkl`, toggle with `Space`/`Enter`, and can expand/collapse with `f`. The selection state is tracked via:
- `SectionChangedLine` structs with `is_checked: bool` flags
- `SelectionKey` enum (File/Section/Line) for focus management
- Tristate checkboxes showing partial selections

### Problem Statement

The current line-level selection is primitive. When making commits, developers often want to select changes at semantic boundaries:
- An entire function/method
- A struct definition with all its fields
- A class with its methods
- Logical blocks within functions (if statements, loops, etc.)

Currently, users must tediously toggle individual lines, which is error-prone and doesn't align with how developers think about code changes.

### Proposed Enhancement

Tree-sitter integration would add a **semantic node level** between sections and lines:

```
File (foo.rs)
  └─ Section (lines 42-89)
      └─ Semantic Node: struct Foo { ... }
          ├─ Field: pub name: String
          ├─ Field: age: u32
          └─ Method: fn new() -> Self { ... }
              └─ Lines within method
```

This would allow users to select/deselect entire functions, methods, struct definitions, or logical blocks rather than tediously toggling individual lines.

## Implementation Complexity Assessment

This is a **complex feature** that requires significant architectural changes.

### Major Challenges

**1. Multi-Language Parser Support**
- Need tree-sitter parsers for each language (Rust, Python, JavaScript, Go, etc.)
- Each language has different grammar rules and semantic constructs
- Runtime parser loading and grammar management
- Dependency size: ~200KB+ per language grammar

**2. Data Model Changes**

The current `Section` enum in `types.rs:432` would need restructuring:

```rust
pub enum Section<'a> {
    Unchanged { lines: Vec<Cow<'a, str>> },
    Changed { lines: Vec<SectionChangedLine<'a>> },
    // NEW: Semantic variant needed
    Semantic {
        node_type: SemanticNodeType,  // Function, Class, Method, etc.
        is_checked: bool,
        is_partial: bool,
        children: Vec<Section<'a>>,  // Recursive structure
        lines: Vec<SectionChangedLine<'a>>,
    },
    // ... existing variants
}
```

**3. UI State Management** (`ui.rs`)
- Extend `SelectionKey` enum with `SemanticNode(SemanticNodeKey)`
- Modify 8+ navigation methods (`select_inner`, `select_outer`, `select_next_of_same_kind`, etc.)
- Update rendering logic to display semantic node boundaries
- Handle partial selections when some lines in a function are selected

**4. Change Attribution**
- Parse both old and new file versions
- Match semantic nodes across diffs (functions may have moved/been renamed)
- Handle partial function changes (only some lines within a function changed)
- Deal with syntactically invalid code (incomplete edits, syntax errors)

**5. Performance Concerns**
- Tree-sitter parsing adds latency to UI initialization
- Large files (>10K lines) may slow down rendering
- Memory overhead for maintaining parse trees

**6. Edge Cases**
- Binary files (no semantic structure)
- File mode changes
- Mixed changes (some semantic, some non-semantic within same section)
- Language detection from file extension
- Fallback when tree-sitter parser unavailable

## Options Considered

### Option 1: Use diffsitter

[diffsitter](https://github.com/afnanenayet/diffsitter) is a semantic diff tool using tree-sitter.

**Pros:**
- ✅ Proven semantic diff attribution
- ✅ Multi-language grammar handling already solved
- ✅ Well-tested change attribution logic

**Cons:**
- ⚠️ Designed for computing diffs, not interactive selection
- ⚠️ Would need significant adaptation for scm-record's UI model
- ⚠️ Additional dependency maintenance burden

**Assessment:** Potentially useful as a reference implementation, but not a drop-in solution.

### Option 2: Use helix-editor infrastructure

[helix-editor](https://github.com/helix-editor/helix) has production-grade tree-sitter integration.

**Pros:**
- ✅ **Production-grade grammar management** (`helix-loader`)
- ✅ **Bundled grammars for 80+ languages**
- ✅ **`hx --grammar {fetch|build}` infrastructure**
- ✅ Well-tested language detection
- ✅ Active maintenance

**Cons:**
- ⚠️ Dependency on helix-loader (MPL-2.0 license)
- ⚠️ Helix's tree-sitter integration designed for text editing, not diff analysis
- ⚠️ Would still need custom semantic tree logic for scm-record

**Assessment:** Strong candidate for grammar management, but semantic layer needs custom implementation.

### Option 3: Use tree-house library

[tree-house](https://github.com/helix-editor/tree-house) is an experimental modern tree-sitter Rust API.

**Pros:**
- ✅ Clean abstraction over tree-sitter-rs
- ✅ Modern, maintained by helix team

**Cons:**
- ⚠️ Experimental/early-stage
- ⚠️ Still requires custom grammar management
- ⚠️ Less mature than helix's built-in infrastructure

**Assessment:** Promising but may not be production-ready.

### Option 4: Build from scratch with tree-sitter-rs

Use the core [tree-sitter](https://crates.io/crates/tree-sitter) crate directly.

**Pros:**
- ✅ Full control over implementation
- ✅ Minimal dependencies
- ✅ Direct integration with scm-record's data model

**Cons:**
- ⚠️ Must implement grammar management from scratch
- ⚠️ Must handle parser loading, language detection
- ⚠️ Significant development effort

**Assessment:** Maximum flexibility but highest implementation cost.

### Option 5: Heuristic-based grouping (lightweight alternative)

Use regex patterns to detect function/class boundaries.

**Pros:**
- ✅ Much simpler implementation
- ✅ No tree-sitter dependency
- ✅ Works immediately across all languages

**Cons:**
- ⚠️ Less accurate than tree-sitter
- ⚠️ Language-specific heuristics required
- ⚠️ Doesn't handle nested structures well

**Assessment:** Simpler but less powerful alternative.

### Option 6: Built-in grammar management with tree-house

Build custom grammar management inspired by helix's approach, but implemented from scratch within scm-record. Use tree-house as the tree-sitter Rust API.

**Pros:**
- ✅ Full control over grammar management
- ✅ No dependencies on helix-loader (avoid tight coupling)
- ✅ Modern tree-sitter API via tree-house
- ✅ Can tailor grammar bundling to scm-record's needs
- ✅ Learn from helix's proven design patterns

**Cons:**
- ⚠️ More implementation work upfront
- ⚠️ Must maintain grammar management code
- ⚠️ tree-house is still experimental

**Assessment:** Best balance of control and modern tooling.

## Decision

**Choose Option 6: Built-in grammar management with tree-house**

We will:
1. Use **tree-house** as the modern tree-sitter Rust API (the gold standard for consuming tree-sitter in Rust)
2. Build **custom grammar management** inspired by helix's philosophy but implemented from scratch
3. **No dependencies** on helix-loader or vendoring of helix code
4. Implement **incrementally**, starting with high-priority languages
5. Make it **opt-in** via feature flag initially

### Architecture

```
scm-record UI → Custom Semantic Builder → tree-house → tree-sitter
                         ↓
                Grammar Management Layer
                (inspired by helix, built in-house)
```

### Why This Approach?

**tree-house advantages:**
- Modern, maintained, actively developed
- Clean abstraction over tree-sitter-rs
- Well-designed for building tools
- Strong community support
- The gold standard for consuming tree-sitter in Rust

**No helix dependencies because:**
- Avoid tight coupling to helix's architecture
- Keep scm-record lightweight and focused
- Maintain full control over grammar management
- Better suited to interactive diff selection vs. text editing

**Custom grammar management because:**
- Full control over which grammars to bundle
- Can optimize for scm-record's specific use case
- Learn from helix's proven patterns without direct dependency
- Flexibility to evolve independently

## Implementation Plan

### Language Priorities

Implement support for languages in this priority order:

1. **Rust** (priority #1 - highest value)
2. **OpenTofu/Terraform**
3. **Markdown**
4. **TypeScript & JavaScript**
5. **HTML**
6. **Svelte**
7. **Python**
8. **Zig**
9. **Swift**
10. **Java**
11. **Lean4**
12. **Haskell**
13. **Kotlin**
14. **Scala**
15. **TOML**
16. **YAML**
17. **JSON**

### Proof of Concept

**Scope:**
- Start with Rust + 2-3 other high-priority languages (e.g., TypeScript, Python)
- Implement minimal grammar management (fetch + compile grammars)
- Build basic semantic tree integration into `Section` enum
- Add UI navigation for semantic nodes
- Must be manually testable

**Success Criteria:**
- Can select/deselect entire Rust functions via keyboard
- Performance acceptable for files up to 1000 lines
- Graceful fallback when parser unavailable

### Phase 1: Foundation

**Tasks:**
- [ ] Add `tree-house` dependency
- [ ] Create `semantic.rs` module for tree-sitter integration
- [ ] Implement grammar management system (fetch, compile, load)
- [ ] Add file language detection (by extension, shebang, etc.)
- [ ] Implement parser loading for initial language set
- [ ] Write unit tests for basic parsing

### Phase 2: Data Model

**Tasks:**
- [ ] Extend `Section` enum with semantic variant
- [ ] Update serialization/deserialization
- [ ] Modify `SectionChangedLine` to track semantic parent
- [ ] Add semantic node metadata (type, name, range)
- [ ] Handle recursive semantic tree structures

### Phase 3: UI Integration

**Tasks:**
- [ ] Add `SemanticNode` variant to `SelectionKey`
- [ ] Extend navigation logic (inner/outer/next/prev)
- [ ] Update rendering to show semantic boundaries
- [ ] Add visual indicators for function/class/method nodes
- [ ] Implement keyboard shortcuts for semantic navigation

### Phase 4: Polish and Testing

**Tasks:**
- [ ] Add configuration option to enable/disable semantic mode
- [ ] Performance optimization for large files
- [ ] Add error handling for invalid syntax
- [ ] Manual testing across supported languages
- [ ] Documentation and examples
- [ ] Consider performance profiling

### Phase 5: Expand Language Support

**Tasks:**
- [ ] Add remaining languages from priority list
- [ ] Test semantic selection across all supported languages
- [ ] Optimize grammar loading for startup performance

## Consequences

### Positive

- Users can select changes at semantic boundaries (functions, classes, methods)
- More natural alignment with how developers think about code
- Reduces error-prone line-by-line selection
- Extensible to support many languages via tree-sitter ecosystem

### Negative

- Increased complexity in data model and UI logic
- Grammar management adds maintenance burden
- Larger binary size due to bundled grammars
- Performance overhead for parsing on large files
- tree-house is experimental and may have breaking changes

### Risks and Mitigations

**Risk:** tree-house has breaking changes
- **Mitigation:** Pin to specific version, monitor upstream closely

**Risk:** Performance degrades on large files
- **Mitigation:** Profile early, implement lazy parsing, add configuration to disable

**Risk:** Grammar management becomes maintenance burden
- **Mitigation:** Automate grammar updates, learn from helix's approach

**Risk:** Users confused by semantic navigation
- **Mitigation:** Make opt-in initially, add clear documentation, visual indicators

## References

- [tree-sitter](https://tree-sitter.github.io/)
- [tree-house (modern tree-sitter Rust API)](https://github.com/helix-editor/tree-house)
- [helix-editor (grammar management inspiration)](https://github.com/helix-editor/helix)
- [diffsitter (semantic diff reference)](https://github.com/afnanenayet/diffsitter)
- Original issue: #2
