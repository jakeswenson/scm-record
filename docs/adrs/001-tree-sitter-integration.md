# ADR 001: Tree-sitter Integration for Semantic-Level Change Selection

## Status

Proposed

## Context

Currently, `scm-record` provides a 3-level hierarchy for interactively selecting changes:

1. **File level** - Select/deselect entire files
2. **Section level** - Select/deselect contiguous blocks of changes
3. **Line level** - Select/deselect individual added/removed lines

While this approach works well, it can be tedious when dealing with large functions or classes where only certain logical components need to be committed. Users must manually select individual lines within a function, rather than selecting the function as a semantic unit.

The proposal is to integrate tree-sitter to add a **semantic node level** between sections and lines, allowing users to select/deselect:
- Entire functions, methods, or classes
- Struct/enum definitions
- Logical code blocks (if/match statements, loops)
- Individual struct fields or enum variants

### Example Use Case

Given a change to a Rust file:

```
File (foo.rs)
  └─ Section (lines 42-89)
      └─ Semantic Node: impl Foo
          ├─ Method: fn new() -> Self { ... }      [select this]
          ├─ Method: fn calculate(&self) { ... }   [skip this]
          └─ Method: fn display(&self) { ... }     [select this]
```

Instead of toggling 20 individual lines for the `new()` and `display()` methods, users could select those two methods as semantic units.

## Decision Drivers

1. **User Experience** - Reduce tedium when selecting logical code units
2. **Code Quality** - Encourage atomic commits of complete semantic changes
3. **Implementation Complexity** - Balance value with development effort
4. **Maintainability** - Leverage existing solutions where possible
5. **Multi-language Support** - Must work across Rust, Python, JavaScript, Go, etc.

## Considered Options

### Option 1: Build Tree-sitter Integration from Scratch

**Approach:** Directly integrate the `tree-sitter` crate and implement all parsing, grammar management, and semantic tree building in-house.

**Pros:**
- Full control over implementation details
- No external dependencies beyond `tree-sitter` crate itself
- Customizable to exact needs

**Cons:**
- Major implementation effort (estimated 6+ weeks)
- Need to solve multi-language parser management
- Grammar loading and versioning complexity
- Reinventing solutions others have already built

### Option 2: Leverage diffsitter for Semantic Diff Parsing

**About diffsitter:** A tool that produces semantic diffs by using tree-sitter to understand code structure. Instead of showing line-based diffs, it shows diffs at the AST node level.

**Reusable Components:**
- Tree-sitter parser initialization and management
- Language detection from file extensions
- AST traversal and diff attribution logic
- Multi-language grammar handling

**Integration Approach:**
```
scm-record → diffsitter parsing → semantic sections → UI rendering
```

**Pros:**
- Proven semantic diff attribution (diffsitter's core competency)
- Already handles multi-language parsing
- Existing grammar management infrastructure
- Active maintenance

**Cons:**
- diffsitter is designed for computing diffs, not interactive selection
- May need to fork/vendor to customize for our use case
- Additional dependency to maintain
- License compatibility needs verification (MIT/Apache-2.0)

### Option 3: Use helix-editor's Tree-sitter Infrastructure

**About helix and tree-house:**
- **helix-editor/helix**: A modern modal text editor with excellent tree-sitter integration
- **helix-editor/tree-house**: An experimental tree-sitter Rust API (note: this project may be in early stages or archived)

**Reusable Components from helix:**

1. **Grammar Management** (`helix-loader`):
   - `hx --grammar fetch` - Downloads grammar repositories
   - `hx --grammar build` - Compiles grammars from source
   - Runtime grammar loading
   - Bundled grammars in the binary

2. **Language Configuration** (`languages.toml`):
   - Language detection patterns
   - Grammar-to-filetype mappings
   - Query configuration for syntax highlighting

3. **Tree-sitter Queries**:
   - Pre-written queries for syntax highlighting, textobjects, etc.
   - Could be adapted for semantic node selection

**Integration Approach:**
```rust
// Reuse helix-loader for grammar management
use helix_loader::{grammar, config};

// Load language config
let lang_config = config::load_language_config()?;

// Get parser for file
let language = grammar::get_language(&lang_config, "rust")?;
let mut parser = tree_sitter::Parser::new();
parser.set_language(language)?;

// Parse and build semantic tree
let tree = parser.parse(source_code, None)?;
```

**Pros:**
- Production-grade grammar management (used daily by helix users)
- Bundled grammars for 80+ languages
- Well-tested language detection
- Active maintenance by helix team
- Could potentially vendor just `helix-loader` crate

**Cons:**
- helix is an editor, not a library - may need significant adaptation
- `helix-loader` may have editor-specific assumptions
- Dependency weight (though we can vendor selectively)
- License compatibility needs verification (MPL-2.0)

### Option 4: Hybrid Approach - Helix Grammar Management + Custom Semantic Layer

**Approach:** Use helix's grammar management infrastructure but build custom semantic tree logic specific to scm-record's needs.

**Architecture:**
```
┌─────────────────────────────────────────┐
│ scm-record UI Layer                     │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│ Custom Semantic Section Builder         │
│ - Maps AST nodes to semantic sections   │
│ - Handles partial selections            │
│ - Integrates with existing Section enum │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│ helix-loader (Grammar Management)       │
│ - Language detection                    │
│ - Grammar loading                       │
│ - Parser initialization                 │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│ tree-sitter (Core Parsing)              │
└─────────────────────────────────────────┘
```

**Implementation Phases:**

1. **Phase 1: Foundation**
   - Vendor or depend on `helix-loader` for grammar management
   - Add configuration for enabling/disabling tree-sitter mode
   - Implement language detection for changed files

2. **Phase 2: Parsing Layer**
   - Integrate parser initialization using helix's grammar loader
   - Parse both old and new versions of changed files
   - Handle parse errors gracefully (fallback to line-based selection)

3. **Phase 3: Semantic Tree Building**
   - Traverse AST to identify semantic nodes (functions, classes, etc.)
   - Map line ranges to semantic nodes
   - Extend `Section` enum with semantic variant

4. **Phase 4: UI Integration**
   - Add `SemanticNode` to `SelectionKey` enum
   - Implement navigation (inner/outer/next/prev)
   - Update rendering to show semantic boundaries
   - Handle partial selections

5. **Phase 5: Multi-language Support**
   - Add language-specific semantic node detection
   - Test with Rust, Python, JavaScript, Go, C++
   - Document language coverage

**Pros:**
- Solves the hardest problem (grammar management) with proven solution
- Custom semantic layer tailored to scm-record's needs
- Incremental implementation path
- Can start with single language, expand gradually
- Lighter weight than depending on all of helix

**Cons:**
- Still significant implementation effort
- Need to maintain vendored helix-loader or track upstream changes
- Custom semantic tree logic is non-trivial

### Option 5: Heuristic-Based Approach (No Tree-sitter)

**Approach:** Use language-specific regex patterns and indentation analysis to group related lines.

**Pros:**
- Much simpler implementation (1-2 weeks vs 6+ weeks)
- No tree-sitter dependency
- Works with any text file

**Cons:**
- Inaccurate for complex code structures
- Brittle and language-specific
- Poor handling of edge cases
- Less valuable to users than true semantic selection

## Decision

**Chosen Option: Option 4 - Hybrid Approach using Helix Grammar Management + Custom Semantic Layer**

### Rationale

This option provides the best balance of:

1. **Leveraging Existing Work**: helix has already solved the hard problem of grammar management, including:
   - Runtime grammar loading
   - Language detection
   - Grammar versioning and updates
   - Cross-platform compilation

2. **Customization**: Building our own semantic layer allows us to:
   - Integrate cleanly with existing `Section` enum
   - Handle partial selections (some lines in a function selected)
   - Optimize for diff viewing vs. general editing

3. **Incremental Path**: We can:
   - Start with Rust support only (dogfood our own use case)
   - Add languages progressively based on user demand
   - Ship value early, iterate based on feedback

4. **Maintainability**:
   - helix-loader is well-maintained and tested
   - Smaller surface area than forking diffsitter
   - Clear separation of concerns

### Implementation Strategy

**Phase 1 (Weeks 1-2): Proof of Concept**
- [ ] Add `helix-loader` dependency (or vendor minimal subset)
- [ ] Implement language detection for Rust files
- [ ] Parse a Rust file and print AST nodes (proof it works)
- [ ] Add feature flag `tree-sitter-semantic` (opt-in)

**Phase 2 (Week 3): Data Model**
- [ ] Extend `Section` enum with `Semantic` variant
- [ ] Add `SemanticNodeType` enum (Function, Impl, Struct, etc.)
- [ ] Implement AST → Section conversion for Rust

**Phase 3 (Week 4): UI Integration**
- [ ] Add `SemanticNode` to `SelectionKey`
- [ ] Implement navigation methods
- [ ] Update rendering to show semantic boundaries
- [ ] Handle expand/collapse of semantic nodes

**Phase 4 (Week 5): Polish**
- [ ] Handle partial selections
- [ ] Performance optimization
- [ ] Error handling and fallbacks
- [ ] Documentation

**Future Work:**
- Add Python support
- Add JavaScript/TypeScript support
- Add Go support
- Explore tree-sitter queries for custom selection patterns

### Alternative: Start with diffsitter

If vendoring/depending on helix-loader proves too complex, we should revisit **Option 2** (diffsitter). diffsitter's semantic diff attribution is directly relevant to our use case, and it may be easier to adapt than helix's editor-centric infrastructure.

## Consequences

### Positive

- **Better User Experience**: Users can select semantic units instead of individual lines
- **Encourages Atomic Commits**: Easier to commit complete functions/classes
- **Multi-language Support**: helix supports 80+ languages out of the box
- **Grammar Updates**: Users can update grammars without updating scm-record
- **Incremental Delivery**: Can ship Rust support first, add languages later

### Negative

- **Complexity**: Additional ~2000-3000 lines of code
- **Dependency**: helix-loader dependency or vendoring burden
- **Performance**: Parsing adds latency to UI initialization (mitigated by caching)
- **Edge Cases**: Incomplete code, syntax errors, mixed changes require fallback logic
- **Maintenance**: Need to track helix-loader updates or maintain vendored code

### Neutral

- **Feature Flag**: tree-sitter mode will be opt-in initially (default: off)
- **Fallback**: Line-based selection remains available for unsupported languages
- **Configuration**: Users can disable tree-sitter if it causes issues

## References

- [diffsitter GitHub Repository](https://github.com/afnanenayet/diffsitter) - Semantic diff tool using tree-sitter
- [helix-editor GitHub Repository](https://github.com/helix-editor/helix) - Modal text editor with excellent tree-sitter support
- [tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/) - Official tree-sitter docs
- [Original Issue Discussion](https://github.com/jakeswenson/scm-record/issues/2) - Community discussion on this feature

## Notes

### License Compatibility

- **scm-record**: Apache-2.0 OR MIT
- **helix**: MPL-2.0 (Mozilla Public License 2.0)
- **diffsitter**: MIT OR Apache-2.0
- **tree-sitter**: MIT

**Verdict**:
- helix's MPL-2.0 license requires that modifications to helix source files be released under MPL-2.0, but allows linking/using as a library without viral effects
- If we vendor helix-loader files, those files must remain MPL-2.0, but our code can remain MIT/Apache-2.0
- diffsitter has compatible MIT/Apache-2.0 dual license

### Performance Considerations

Parsing large files (>10K lines) can add latency. Mitigation strategies:

1. **Lazy Parsing**: Only parse files when user expands them
2. **Caching**: Cache parse trees for files
3. **Background Parsing**: Parse in background thread, show line-based UI immediately
4. **Size Limits**: Disable tree-sitter for files >50K lines

### Grammar Management Options

If helix-loader proves difficult to integrate, alternatives:

1. **Bundle Grammars**: Ship pre-compiled grammars in scm-record binary
2. **Grammar Registry**: Maintain our own grammar registry/loader
3. **User-Provided**: Let users configure grammar paths (like helix's runtime dir)

## Decision Date

2025-10-25

## Approval

This ADR is proposed for discussion and approval by the project maintainers and community.
