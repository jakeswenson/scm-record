# syntax-diff-editor

[Build status]: https://img.shields.io/github/actions/workflow/status/jakeswenson/syntax-diff-editor/.github%2Fworkflows%2Flinux.yml
[link-build-status]: https://github.com/jakeswenson/syntax-diff-editor/actions?branch=main
[Latest version]: https://img.shields.io/crates/v/syntax-diff-editor.svg
[link-latest-version]: https://crates.io/crates/syntax-diff-editor
[Docs]: https://img.shields.io/docsrs/syntax-diff-editor
[link-docs]: https://docs.rs/syntax-diff-editor/latest/syntax_diff_editor/
[License]: https://img.shields.io/crates/l/syntax-diff-editor
[link-license]: https://github.com/jakeswenson/syntax-diff-editor

[![Build status]][link-build-status] [![Latest version]][link-latest-version] [![Docs]][link-docs] [![License]][link-license]

`syntax-diff-editor` is a terminal-based diff and merge tool with semantic navigation powered by tree-sitter. It provides an interactive interface for reviewing and selecting changes, with the ability to navigate by code structure (functions, classes, methods) rather than just diff hunks.

## Overview

Think of this as an enhanced interactive replacement for `git add -p` or `hg crecord`/`hg commit -i`, with syntax-aware navigation. Given a set of changes, this tool presents them in an interactive interface where you can:

- Navigate through files, functions, and individual changes
- Select/deselect changes at any granularity (files, sections, or individual lines)
- Review changes with syntax highlighting
- Navigate by semantic structure using tree-sitter

### Key Features

- **Semantic Navigation**: Navigate by code structure (functions, classes, methods) using tree-sitter integration, not just diff hunks
- **Flexible Selection**: Toggle individual lines, entire sections, or whole files
- **Keyboard-Driven**: Vim-like keybindings for efficient navigation and selection
- **Mouse Support**: Click to focus, scroll to navigate
- **Universal Compatibility**: Works as a difftool or mergetool with any source control system

### About This Fork

**Important Note**: `syntax-diff-editor` represents a significant change from the original `scm-diff-editor` project. The original `scm-diff-editor` (and the underlying `scm-record` library) is an absolutely wonderful piece of work. The tree-based selection approach for navigating and selecting changes is ergonomically brilliant - it makes reviewing and staging changes feel natural and intuitive in a way that's hard to describe until you experience it.

This fork exists primarily as an experiment: I was so impressed with the elegance of the tree selection interface that I wanted to explore enhancing it with semantic/syntax-aware navigation via tree-sitter. The rename to `syntax-diff-editor` reflects this focus on syntax-driven navigation, allowing you to navigate your diffs by functions, classes, and methods rather than just by hunks.

All credit for the core design and implementation goes to the original authors. This is merely an exploration of "what if we could navigate by code structure too?" built on top of their excellent foundation.

**Original Project**: [arxanas/scm-record](https://github.com/arxanas/scm-record)

### Installation

Install directly from the git repository:

```sh
cargo install --git https://github.com/jakeswenson/syntax-diff-editor.git syntax-diff-editor
```

Or install from crates.io once published:

```sh
cargo install --locked syntax-diff-editor
```

### Usage

The `syntax-diff-editor` executable can be used with:

- **[Git](https://git-scm.org)**: as a [difftool](https://git-scm.com/docs/git-difftool) or mergetool
- **[Mercurial](https://www.mercurial-scm.org/)**: via [the `extdiff` extension](https://wiki.mercurial-scm.org/ExtdiffExtension)
- **Any source control system** that supports external diff/merge tools

#### Basic Usage

```sh
# Compare two files
syntax-diff-editor file1.txt file2.txt

# Compare two directories (auto-detected)
syntax-diff-editor dir1/ dir2/

# Compare two directories (explicit flag)
syntax-diff-editor --dir-diff dir1/ dir2/
```

**Note**: Directory mode is automatically detected when both paths are directories. You can use the `--dir-diff` flag to make the behavior explicit or for use in scripts.

#### Git Integration

Example Git configuration:

```sh
# Set as difftool
git config --global diff.tool syntax-diff-editor
git config --global difftool.syntax-diff-editor.cmd 'syntax-diff-editor "$LOCAL" "$REMOTE"'

# Use it
git difftool
```

## Keyboard Controls

`syntax-diff-editor` provides a rich set of keyboard shortcuts for efficient navigation and selection. Key bindings include:

- **Navigation**: `j/k` (or arrow keys) for up/down, `h/l` for in/out of hierarchy
- **Selection**: `Space` to toggle, `Enter` to toggle and advance, `a` for select/deselect all
- **Commit**: `c` to accept changes, `e` to edit commit message, `q` to quit

For a complete list of keyboard shortcuts and navigation tips, see the [Keyboard Controls documentation](docs/keyboard-controls.md).

## Contributing

Contributions are welcome! Here are some areas where `syntax-diff-editor` could be improved:

### Feature Wishlist

- Improve semantic navigation for more languages (expand tree-sitter language support)
- Make keybindings easier to discover
- Support accessing the menu with the keyboard
- Edit one side of the diff in an editor
- Multi-way split UI to split a commit into more than 2 commits
- Full mergetool support with conflict resolution commands
- Commands to select ours/theirs for merge conflicts

## Documentation

- [Keyboard Controls](docs/keyboard-controls.md) - Complete reference for all keyboard shortcuts
- [ADR: Tree-sitter Integration](docs/adrs/001-tree-sitter-integration.md) - Design decisions for semantic navigation

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
