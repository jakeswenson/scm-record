# ADR 001: Tree-sitter Integration for Semantic-Level Change Selection

## Status

Proposed

## Context

### Current State (Baseline - Pre-Semantic)

**scm-record** currently uses a **diff-first** 3-level hierarchy for selection:

1. **File** level (e.g., `foo/bar.rs`)
2. **Section** level (contiguous blocks of changes from the unified diff)
3. **Line** level (individual added/removed lines within a section)

#### How Current Sectioning Works

Sections are determined purely by the **unified diff format**. A section is a contiguous block of changes with surrounding context lines. For example:

```diff
@@ -10,7 +10,8 @@ impl Foo {
     fn calculate(&self) -> i32 {
         let x = self.field1;
-        let y = x * 2;
+        let y = x * 3;
+        let z = y + 10;
         return z;
     }
@@ -42,5 +43,5 @@ impl Bar {
     fn process(&mut self) {
-        self.count += 1;
+        self.count += 2;
     }
```

This diff creates **two sections**:
- Section 1: Lines 10-17 (changes in `Foo::calculate`)
- Section 2: Lines 42-47 (changes in `Bar::process`)

The sections have **no semantic awareness** - they're purely based on diff proximity, not code structure.

#### Current User Experience

Users navigate with `hjkl`, toggle with `Space`/`Enter`, and expand/collapse with `f`. Selection state is tracked via:
- `SectionChangedLine` structs with `is_checked: bool` flags
- `SelectionKey` enum (File/Section/Line) for focus management
- Tristate checkboxes showing partial selections

**Pain point:** If a single method has changes in multiple non-contiguous locations, they appear as **separate sections** even though they're semantically part of the same method. Users must manually find and toggle each section.

### Problem Statement

The current line-level selection is primitive. When making commits, developers often want to select changes at semantic boundaries:
- An entire function/method
- A struct definition with all its fields
- A class with its methods
- Logical blocks within functions (if statements, loops, etc.)

Currently, users must tediously toggle individual lines, which is error-prone and doesn't align with how developers think about code changes.

### Proposed Enhancement: Semantic-First Navigation

Tree-sitter integration would **replace** diff-first sectioning with a **semantic-first hierarchy**:

```
File (foo.rs)
  ├─ Container: struct Foo { ... }
  │   ├─ Field: pub name: String
  │   │   └─ Section (lines 10-12) - diff block with field change
  │   └─ Field: age: u32
  │       └─ Section (lines 15-17) - diff block with field change
  ├─ Container: impl Foo { ... }
  │   ├─ Method: fn new() -> Self { ... }
  │   │   └─ Section (lines 45-50) - diff block within method
  │   └─ Method: fn calculate(&self) -> i32 { ... }
  │       ├─ Section (lines 60-65) - first diff block in method
  │       └─ Section (lines 70-75) - second diff block in method
  └─ Container: Top-level function: fn helper() { ... }
      └─ Section (lines 100-105) - diff block within function
```

**Key differences from current approach:**

1. **Containers are primary grouping** - Changes are organized by semantic containers (struct, impl, functions) rather than by diff proximity
2. **Members within containers** - Methods, fields within their parent containers
3. **Sections as leaf nodes** - Traditional diff sections become the finest-grained navigation level under their semantic parent
4. **Cross-version matching** - Containers/members matched between old and new file versions by name
5. **Hierarchical selection** - Selecting a container selects all its members and sections

**Benefits:**
- Navigate to "all changes in struct Foo" with one selection
- Select entire method with all its scattered diff sections
- Intuitive grouping aligned with code structure
- Reduces error-prone line-by-line toggling

## Implementation Complexity Assessment

This is a **complex feature** that requires significant architectural changes.

### Major Challenges

**1. Multi-Language Parser Support**
- Need tree-sitter parsers for each language (Rust, Python, JavaScript, Go, etc.)
- Each language has different grammar rules and semantic constructs
- Runtime parser loading and grammar management
- Dependency size: ~200KB+ per language grammar

**2. Data Model Changes**

The current `Section` enum in `types.rs:432` would need **complete restructuring** from diff-first to semantic-first:

**Current structure (diff-first):**
```rust
File → Vec<Section> → Vec<SectionChangedLine>
```

**New structure (semantic-first):**
```rust
pub struct File<'a> {
    path: PathBuf,
    containers: Vec<SemanticContainer<'a>>,
    // Fallback for non-semantic code or parse failures
    fallback_sections: Option<Vec<Section<'a>>>,
}

pub enum SemanticContainer<'a> {
    Struct {
        name: String,
        fields: Vec<SemanticMember<'a>>,
        is_checked: bool,
        is_partial: bool,
    },
    Impl {
        type_name: String,  // "Foo" for "impl Foo"
        trait_name: Option<String>,  // Some("Display") for "impl Display for Foo"
        methods: Vec<SemanticMember<'a>>,
        is_checked: bool,
        is_partial: bool,
    },
    Function {
        name: String,
        sections: Vec<Section<'a>>,
        is_checked: bool,
        is_partial: bool,
    },
}

pub enum SemanticMember<'a> {
    Field {
        name: String,
        sections: Vec<Section<'a>>,  // Diff blocks for this field
        is_checked: bool,
        is_partial: bool,
    },
    Method {
        name: String,
        sections: Vec<Section<'a>>,  // Diff blocks within this method
        is_checked: bool,
        is_partial: bool,
    },
}

// Section becomes the leaf node containing actual diff lines
pub struct Section<'a> {
    lines: Vec<SectionChangedLine<'a>>,
    is_checked: bool,
    is_partial: bool,
}
```

**Key changes:**
- File contains **containers** (struct/impl/function) not raw sections
- Containers have **members** (fields/methods)
- Members have **sections** (traditional diff blocks)
- **Fallback path**: If semantic parsing fails, use `fallback_sections` with current diff-first behavior

**3. UI State Management** (`ui.rs`)
- Extend `SelectionKey` enum with `SemanticNode(SemanticNodeKey)`
- Modify 8+ navigation methods (`select_inner`, `select_outer`, `select_next_of_same_kind`, etc.)
- Update rendering logic to display semantic node boundaries
- Handle partial selections when some lines in a function are selected

**4. Change Attribution and Cross-Version Matching**

This is **critical** for semantic-first navigation:

- **Parse both old and new file versions** with tree-sitter
- **Match containers/members across versions** to attribute changes correctly
- **Initial approach**: Name-based matching
  - `struct Foo` in old version matches `struct Foo` in new version
  - `impl Foo::calculate` in old matches `impl Foo::calculate` in new
  - Renamed/moved code appears as delete + add (acceptable for POC)
- **Future enhancement**: Rename tracking and structural similarity matching
- **Handle partial changes**: Method exists in both versions but only some lines changed
- **Handle additions/deletions**: New structs/methods or removed ones
- **Deal with syntactically invalid code**: Fallback to diff-first when parse fails
- **Containers as separate units**: In Rust, `struct Foo` and `impl Foo` matched independently (not grouped)

**5. Performance Concerns**
- Tree-sitter parsing adds latency to UI initialization
- Large files (>10K lines) may slow down rendering
- Memory overhead for maintaining parse trees

**6. Edge Cases and Fallback Strategy**
- **Binary files**: No semantic structure → fallback to diff-first sectioning
- **Unsupported languages**: Parser not available → fallback to diff-first sectioning
- **Syntax errors**: Invalid code that tree-sitter can't parse → fallback to diff-first sectioning
- **File mode changes**: No code content → fallback to diff-first sectioning
- **Top-level code**: Module imports, constants, type aliases → fallback to diff-first sectioning at file level
- **Language detection**: By file extension, shebang, or content analysis
- **Graceful degradation**: Semantic parsing failure should never break the UI

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

**Choose Option 4 (Modified): Build from scratch with tree-sitter + bundled grammar crates**

We will:
1. Use **tree-sitter** core crate directly with individual language grammar crates from crates.io
2. Bundle **First Wave grammars** as built-in optional dependencies via `tree-sitter` feature flag
3. **No external grammar fetching** - all grammars bundled with the binary from crates.io
4. **No dependencies** on helix-loader, tree-house, or other abstraction layers
5. Implement **incrementally**, starting with First Wave languages (Rust, Kotlin, Java, HCL, Python, Markdown, TOML, YAML)
6. Make it **opt-in** via `tree-sitter` feature flag

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      scm-record UI                           │
│              (File → Container → Member → Section)           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
         ┌─────────────────────────────┐
         │  Semantic Builder Module    │
         │  (semantic.rs)               │
         │  - Detect language           │
         │  - Parse old & new versions │
         │  - Match containers/members │
         │  - Attribute changes         │
         │  - Build semantic hierarchy  │
         └─────────┬─────────┬─────────┘
                   │         │
         Semantic  │         │  Fallback (parse failure,
          Success  │         │   unsupported language)
                   │         │
                   ▼         ▼
         ┌─────────────────────────┐  ┌──────────────────┐
         │ tree-sitter parsers     │  │  Diff-First      │
         │ (bundled from crates.io)│  │  Sectioning      │
         │ - tree-sitter-rust      │  │  (traditional)   │
         │ - tree-sitter-kotlin-ng │  └──────────────────┘
         │ - tree-sitter-java      │
         │ - tree-sitter-hcl       │
         │ - tree-sitter-python    │
         │ - tree-sitter-md        │
         │ - tree-sitter-toml      │
         │ - tree-sitter-yaml      │
         └─────────────────────────┘
```

**Data Flow:**
1. UI receives diff from version control
2. Semantic Builder detects language by file extension
3. If language supported: Parse both old and new file versions with appropriate tree-sitter parser
4. If successful: Match containers/members, build semantic hierarchy
5. If failed or unsupported: Fall back to traditional diff-first sectioning
6. UI renders either semantic-first or diff-first hierarchy

### Why This Approach?

**Bundled crates.io grammars advantages:**
- **Zero external fetching**: All grammars bundled as Cargo dependencies
- **Reliable builds**: No network dependency during compilation
- **Version-locked**: Grammar versions pinned in Cargo.toml
- **Minimal complexity**: No custom grammar management layer needed
- **Cargo feature flags**: Users can opt-in via `tree-sitter` feature
- **Standard Rust ecosystem**: Uses normal dependency resolution

**Direct tree-sitter usage:**
- **Full control**: Direct API access without abstraction layers
- **Minimal dependencies**: Only tree-sitter + language crates
- **Well-documented**: tree-sitter API is stable and well-documented
- **No experimental dependencies**: Avoid early-stage abstractions like tree-house

**Incremental rollout:**
- **Start small**: First Wave covers 8 core languages
- **Prove value**: Get semantic navigation working for high-value languages first
- **Expand later**: Add Second Wave and Future Waves as needed
- **Low risk**: Feature flag allows disabling if issues arise

### Semantic-First Design Decisions

**Container Grouping Strategy:**
- **Rust**: `struct Foo` and `impl Foo` are **separate containers** (not grouped together)
  - `struct Foo { }` is one container with its fields
  - `impl Foo { }` is a separate container with its methods
  - `impl Display for Foo { }` is yet another separate container
  - This maintains tree-sitter's natural AST structure
- **Top-level functions**: Each function is its own **container**
- **Top-level non-functional code**: Imports, constants, type aliases fall back to diff-first sectioning

**Matching Strategy:**
- **Initial implementation**: Name-based matching
  - `struct Foo` matches by name "Foo"
  - `impl Foo::method_name` matches by type + method name
  - Renamed/moved code shows as delete + add (acceptable for POC)
- **Future enhancement**: Structural similarity and rename tracking

**Fallback Strategy:**
- **Always available**: Traditional diff-first sectioning as fallback
- **Triggers**: Unsupported language, syntax errors, parse failures, binary files
- **Graceful degradation**: Semantic parsing failure never breaks the UI
- **Mixed mode**: File can have both semantic containers (for parsed code) and fallback sections (for unparsed code)

## Implementation Plan

### Language Priorities

Languages will be implemented in waves, starting with those available on crates.io as built-in dependencies.

#### First Wave: Built-in Support (from crates.io)

These languages will be **bundled with the binary** as optional dependencies via the `tree-sitter` feature flag:

1. **Rust** - `tree-sitter-rust = "0.24"` (priority #1 - highest value)
2. **Kotlin** - `tree-sitter-kotlin-ng = "1.1.0"`
3. **Java** - `tree-sitter-java = "0.23.5"`
4. **HCL** (Terraform/OpenTofu) - `tree-sitter-hcl = "1.1"`
5. **Python** - `tree-sitter-python = "0.25"`
6. **Markdown** - `tree-sitter-md = "0.5.1"`
7. **TOML** - `tree-sitter-toml = "0.20.0"`
8. **YAML** - `tree-sitter-yaml = "0.7.2"`

**Rationale:** These languages are:
- Available on crates.io with stable versions
- Cover primary use cases (systems programming, config files, documentation, general-purpose languages)
- Can be built and distributed without external grammar fetching

#### Second Wave: Additional Languages

9. **Swift** - `tree-sitter-swift = "0.7.1"` (tier-two grammar, not in tree-sitter-grammars tier-one)
10. **TypeScript & JavaScript**
11. **HTML**
12. **Svelte**

#### Future Waves

- **Zig**
- **Lean4**
- **Haskell**
- **Scala**
- **JSON**
- Others as needed

### Proof of Concept

**Scope:**
- Start with **First Wave languages** (Rust, Kotlin, Java, HCL, Python, Markdown, TOML, YAML)
- All grammars available from crates.io as built-in dependencies
- Build semantic-first data model (File → Container → Member → Section)
- Implement cross-version matching (name-based initially)
- Add UI navigation for semantic hierarchy
- Implement fallback to diff-first sectioning
- Must be manually testable

**Success Criteria:**
- Can select/deselect entire Rust structs, impls, and functions via keyboard
- Can navigate hierarchically (File → Container → Member → Section)
- Performance acceptable for files up to 1000 lines
- Graceful fallback to diff-first sectioning when parser fails or language unsupported

### Phase 1: Foundation

**Tasks:**
- [ ] Add First Wave tree-sitter language crates as optional dependencies (already in Cargo.toml)
- [ ] Create `semantic.rs` module for tree-sitter integration
- [ ] Implement language detection (by file extension: .rs, .kt, .java, .tf, .py, .md, .toml, .yaml/.yml)
- [ ] Implement parser initialization for First Wave languages
- [ ] Build cross-version parsing (parse both old and new file versions)
- [ ] Write unit tests for basic parsing and language detection

### Phase 2: Data Model

**Tasks:**
- [ ] Create `SemanticContainer` enum (Struct/Impl/Function variants)
- [ ] Create `SemanticMember` enum (Field/Method variants)
- [ ] Restructure `File` to support both containers and fallback_sections
- [ ] Update `Section` to be a leaf node in the hierarchy
- [ ] Add selection state tracking (is_checked, is_partial) at each level
- [ ] Update serialization/deserialization for new data model
- [ ] Implement hierarchical selection propagation (selecting container selects all members)

### Phase 3: UI Integration

**Tasks:**
- [ ] Extend `SelectionKey` enum with Container/Member/Section variants
- [ ] Update navigation logic to traverse semantic hierarchy (File → Container → Member → Section)
- [ ] Implement `select_inner`/`select_outer` for hierarchical navigation
- [ ] Update rendering to show semantic boundaries and structure
- [ ] Add visual indicators for containers (struct/impl/function), members (method/field), and sections
- [ ] Implement hierarchical selection toggling (Space/Enter on container selects all members)
- [ ] Update expand/collapse logic to work with semantic hierarchy
- [ ] Handle mixed mode rendering (semantic containers + fallback sections)

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

## Fallback Strategy

The semantic-first navigation is **optional** and will gracefully fall back to the traditional diff-first sectioning in several scenarios:

### When Fallback Occurs

1. **Unsupported Language**: File extension doesn't match any available tree-sitter grammar
2. **Parser Unavailable**: Grammar not installed or failed to load
3. **Syntax Errors**: Code contains parse errors (incomplete edits, invalid syntax)
4. **Binary Files**: Non-text files with no semantic structure
5. **File Mode Changes**: Permissions or mode changes without content changes
6. **Parse Timeout**: Parsing takes too long (performance safeguard)

### How Fallback Works

**Data Model:**
```rust
pub struct File<'a> {
    path: PathBuf,
    // Semantic containers (if parsing succeeded)
    containers: Vec<SemanticContainer<'a>>,
    // Fallback sections (if parsing failed or for non-semantic code)
    fallback_sections: Option<Vec<Section<'a>>>,
}
```

**Rendering Logic:**
- If `containers` is non-empty: Render semantic-first hierarchy
- If `containers` is empty but `fallback_sections` is present: Render traditional diff-first sections
- **Mixed mode possible**: Some containers parsed semantically, other code falls back

**User Experience:**
- Fallback is **transparent** - UI still works, just without semantic grouping
- No error messages for normal fallback scenarios
- Configuration option to disable semantic parsing entirely (always use diff-first)

### Top-Level Code Handling

For code that doesn't fit into semantic containers:
- **Module imports** (`use foo::bar;`) → Fallback sections at file level
- **Module constants** (`const MAX: usize = 100;`) → Fallback sections at file level
- **Type aliases** (`type Result<T> = ...;`) → Fallback sections at file level
- **Top-level functions**: Treated as semantic **containers** (each function is a container)

This ensures all code is always selectable, whether semantic parsing succeeds or not.

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
