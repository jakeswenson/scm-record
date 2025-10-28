# Keyboard Controls

This document describes all keyboard shortcuts and controls available in `scm-record` and `syntax-diff-editor`.

## Basic Navigation

### Vertical Movement
- **`j` / `↓`** - Move to next item
- **`k` / `↑`** - Move to previous item
- **`Ctrl+d`** - Move down half a page
- **`Ctrl+u`** - Move up half a page
- **`PageDown`** - Jump to next item of same kind (e.g., file to file, section to section)
- **`PageUp`** - Jump to previous item of same kind

### Horizontal Movement (Hierarchy Navigation)
- **`l` / `→`** - Expand/enter current item (e.g., expand a file to see sections)
- **`h` / `←`** - Collapse/exit current item
- **`Shift+h` / `Shift+←`** - Exit current item without collapsing

When using semantic navigation (tree-sitter feature):
- **`l` / `→`** on a collapsed container (e.g., `impl`, `struct`) will expand it to show methods/fields
- **`l` / `→`** on a collapsed method/field will expand it to show diff sections
- **`h` / `←`** collapses the current level and moves focus to the parent

### Scrolling
- **`Ctrl+e`** - Scroll down one line
- **`Ctrl+y`** - Scroll up one line
- **`Ctrl+f`** - Scroll down one page
- **`Ctrl+b`** - Scroll up one page
- **Mouse wheel** - Scroll up/down

## Selection and Toggling

### Toggle Individual Items
- **`Space`** - Toggle current item (select/deselect)
- **`Enter`** - Toggle current item and advance to next

### Bulk Operations
- **`a`** - Toggle all items (if some selected, deselect all; if none selected, select all)
- **`A` (Shift+a)** - Toggle all items uniformly (force all to same state)

### Expanding/Collapsing
- **`f`** - Expand current item to show all nested content
- **`F` (Shift+f)** - Expand all items in the view

## Commit Operations

- **`c`** - Accept and commit selected changes
- **`e`** - Edit commit message
- **`q`** - Cancel and quit without committing
- **`Esc`** - Quit (escape)
- **`Ctrl+c`** - Force quit (interrupt)

## Help

- **`?`** - Show help screen with key bindings

## Mouse Support

- **Left click** - Focus on clicked item
- **Scroll wheel** - Scroll up/down

## Navigation Hierarchy

The navigation system follows a hierarchical structure:

```
Files
  ├─ Semantic Containers (when tree-sitter is enabled)
  │   ├─ Functions/Structs/Impls/Classes/etc.
  │   └─ Methods/Fields (nested within containers)
  │       └─ Sections
  │           └─ Lines
  └─ Sections (when no semantic containers)
      └─ Lines
```

### Semantic Navigation (Tree-Sitter Feature)

When the `tree-sitter` feature is enabled, files are parsed to extract semantic structure:

- **Containers start collapsed** - `impl` blocks, `struct` definitions, `class` declarations, etc. are collapsed by default
- **Methods/Fields start collapsed** - Individual methods and fields within containers are also collapsed
- **Sections auto-expand** - When you expand a method/field, its diff sections are immediately visible

This allows you to navigate by semantic structure (e.g., jump between functions) rather than just by diff sections.

Example workflow:
1. Open a file (collapsed by default)
2. Press `l` to expand the file → shows collapsed containers (functions, impls, etc.)
3. Navigate to an `impl` block with `j`/`k`
4. Press `l` to expand the impl → shows collapsed methods
5. Navigate to a method with `j`/`k`
6. Press `l` to expand the method → shows diff sections immediately
7. Toggle individual sections with `Space` or lines with `Enter`

## Tips

- Use `PageUp`/`PageDown` to quickly jump between items of the same type (e.g., skip from function to function)
- Use `Shift+h` when you want to navigate up the hierarchy without collapsing the current level
- Use `f` on a deeply nested item to expand all its children at once
- Use `a` multiple times to quickly select/deselect everything
