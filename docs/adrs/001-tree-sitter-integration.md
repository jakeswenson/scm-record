# ADR 001: Tree-sitter Integration for Semantic-Level Change Selection

## Status

Proposed

## Context

scm-record currently provides a 3-level hierarchy for selecting changes:
1. **File** level (e.g., `foo/bar.rs`)
2. **Section** level (contiguous blocks of changes)
3. **Line** level (individual added/removed lines)

While this works well for basic use cases, selecting related logical units (like entire functions, classes, or methods) requires tediously toggling individual lines. Users have requested the ability to select changes at a semantic level using tree-sitter to parse code structure.

### Current Limitations

- No awareness of code structure (functions, classes, methods)
- Must manually select all lines belonging to a logical unit
- Cannot easily select/deselect entire semantic constructs
- No language-aware grouping of changes

### Desired Behavior

Users should be able to:
- Select/deselect entire functions, methods, classes, or other semantic units
- Navigate between semantic boundaries (jump to next function, etc.)
- See visual indicators for semantic structure in the UI
- Fall back to line-level selection when semantic parsing fails

## Decision

We will integrate tree-sitter into scm-record using the **tree-house** library, implementing a custom grammar management and semantic selection system built into this application (no external dependencies on helix-editor or similar tools).

### Approach: Built-in Grammar Management (Similar to Helix Philosophy)

**Use tree-house as the modern, maintained API for consuming tree-sitter in Rust.**

We will implement our own grammar management infrastructure inspired by helix's approach:
- Bundle pre-compiled grammars for common languages
- Support runtime grammar fetching/compilation (similar to `hx --grammar {fetch|build}`)
- Maintain our own language configuration and detection
- Build semantic tree logic tailored to scm-record's interactive selection needs

**Why tree-house?**
- Modern, maintained, and actively developed tree-sitter Rust API
- Clean abstraction over tree-sitter-rs
- Well-designed for building tools on top of tree-sitter
- Strong community support and documentation

**Why not depend on helix-loader or vendor helix code?**
- Avoid tight coupling to helix's architecture and release cycle
- Keep scm-record lightweight and focused
- Maintain full control over grammar management
- Simpler dependency tree
- Better suited to scm-record's specific use case (interactive diff selection vs. text editing)

### Architecture

```
scm-record UI → Custom Semantic Builder → tree-house → tree-sitter
                         ↓
                Grammar Management Layer
                (inspired by helix, built in-house)
```

### Key Components

1. **Grammar Management** (`grammar.rs`)
   - Language detection from file extensions
   - Grammar loading (bundled or runtime-compiled)
   - Grammar registry and caching
   - Configuration for language-specific rules

2. **Semantic Tree Builder** (`semantic.rs`)
   - Parse file contents using tree-house
   - Build semantic tree from parse results
   - Map changed lines to semantic nodes
   - Handle partial selections and invalid syntax

3. **Data Model Updates** (`types.rs`)
   - Extend `Section` enum with semantic variant
   - Add `SemanticNode` type (function, class, method, etc.)
   - Track semantic parent-child relationships
   - Support partial semantic selections

4. **UI Integration** (`ui.rs`)
   - Add `SemanticNode` variant to `SelectionKey`
   - Extend navigation (inner/outer/next/prev for semantic units)
   - Render semantic boundaries and indicators
   - Provide keyboard shortcuts for semantic navigation

## Consequences

### Positive

- **Full control**: We own the grammar management and can optimize for our use case
- **Modern API**: tree-house provides a clean, maintained interface to tree-sitter
- **Flexibility**: Can implement exactly the features we need without compromise
- **Lightweight**: No unnecessary dependencies on large editor frameworks
- **Maintainability**: Clear separation of concerns, easier to debug and extend

### Negative

- **Implementation effort**: Must build grammar management from scratch (though inspired by proven approaches)
- **Maintenance burden**: Responsible for keeping grammars up-to-date
- **Testing complexity**: Need comprehensive tests for multi-language support
- **Binary size**: Bundled grammars will increase binary size (mitigated by optional feature flags)

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Grammar compilation complexity | Provide pre-compiled grammars for common languages, make compilation optional |
| Multi-language support burden | Start with priority languages, add more incrementally based on user demand |
| Performance on large files | Implement lazy parsing, caching, and configurable depth limits |
| Syntax errors breaking parsing | Graceful fallback to line-level selection when parsing fails |
| Binary size bloat | Use feature flags to make language support optional |

## Implementation Plan

### Phase 1: Foundation
- [ ] Add tree-house dependency
- [ ] Create grammar management infrastructure (inspired by helix)
- [ ] Implement language detection
- [ ] Add support for Rust grammar (priority #1)
- [ ] Write unit tests for parsing and grammar loading

### Phase 2: Minimal Language Set
Priority languages for initial support (in order):
1. Rust
2. OpenTofu/Terraform
3. Markdown
4. TypeScript & JavaScript
5. HTML
6. Svelte
7. Python
8. Zig
9. Swift
10. Java
11. Lean4
12. Haskell
13. Kotlin
14. Scala
15. TOML
16. YAML
17. JSON

Start with the minimal set needed for manual testing (Rust + 2-3 others).

### Phase 3: Data Model
- [ ] Extend `Section` enum with semantic variant
- [ ] Add `SemanticNode` types and metadata
- [ ] Update serialization/deserialization
- [ ] Track semantic parent-child relationships

### Phase 4: UI Integration
- [ ] Add `SemanticNode` to `SelectionKey`
- [ ] Implement semantic navigation (inner/outer/next/prev)
- [ ] Render semantic boundaries in UI
- [ ] Add visual indicators for semantic nodes
- [ ] Provide keyboard shortcuts

### Phase 5: Polish and Testing
- [ ] Add configuration option to enable/disable semantic mode
- [ ] Performance optimization for large files
- [ ] Comprehensive error handling
- [ ] Manual testing with real-world codebases
- [ ] Documentation and examples

### Proof of Concept Requirements

The initial proof of concept should:
1. Support Rust and a minimal set of the priority languages (e.g., Rust + TypeScript + Python)
2. Allow manual testing of semantic selection
3. Demonstrate the core functionality:
   - Parse files and identify semantic nodes (functions, classes, methods)
   - Display semantic boundaries in the UI
   - Allow selection/deselection of entire semantic units
   - Fall back gracefully when parsing fails

## Alternatives Considered

### Option A: Use diffsitter
**Pros:**
- Proven semantic diff attribution
- Multi-language grammar handling already solved
- Battle-tested in production

**Cons:**
- Designed for computing diffs, not interactive selection
- Would require significant adaptation for our use case
- Additional dependency on external tool's architecture

**Decision:** Rejected. While diffsitter has good ideas, it's optimized for a different use case.

### Option B: Depend on helix-loader
**Pros:**
- Production-grade grammar management
- 80+ bundled grammars
- Well-tested language detection
- Proven `--grammar {fetch|build}` infrastructure

**Cons:**
- Tight coupling to helix's architecture
- Unnecessary dependencies for a diff tool
- Less control over grammar management
- MPL-2.0 license (though compatible)

**Decision:** Rejected. We prefer to learn from helix's approach but build our own implementation.

### Option C: Vendor helix code
**Pros:**
- Can pick specific components
- No runtime dependency on helix releases

**Cons:**
- Maintenance burden (must manually sync updates)
- License attribution requirements
- Still tightly coupled to helix's code style

**Decision:** Rejected. Building our own is cleaner.

### Option D: Heuristic-based grouping (regex)
**Pros:**
- Much simpler implementation
- No parser dependencies
- Works immediately

**Cons:**
- Language-specific regex patterns needed
- Fragile and error-prone
- Cannot handle complex nesting
- No semantic understanding

**Decision:** Rejected. Not robust enough for production use.

### Option E: Indentation-based blocks
**Pros:**
- Language-agnostic
- Very simple to implement
- No parsing required

**Cons:**
- Doesn't understand semantic structure
- Fails for inconsistent indentation
- Not meaningful for many languages (e.g., Go, C)

**Decision:** Rejected. Too simplistic for our needs.

## References

- [tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/)
- [tree-house library](https://github.com/helix-editor/tree-house)
- [helix-editor grammar management](https://github.com/helix-editor/helix/tree/master/helix-loader)
- [diffsitter project](https://github.com/afnanenayet/diffsitter)
- [Original feature request](https://github.com/jakeswenson/scm-record/issues/2)

## Notes

- This feature should be opt-in initially via feature flag to avoid breaking existing users
- Consider proposing this to upstream (arxanas/scm-record) once proven
- Will benefit the entire ecosystem (git-branchless, Jujutsu)
- Grammar compilation should be optional; provide pre-compiled grammars for common use
