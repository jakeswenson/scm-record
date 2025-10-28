# scm-record

[Build status]: https://img.shields.io/github/actions/workflow/status/arxanas/scm-record/.github%2Fworkflows%2Flinux.yml
[link-build-status]: https://github.com/arxanas/scm-record/actions?branch=main
[Latest version]: https://img.shields.io/crates/v/scm-record.svg
[link-latest-version]: https://crates.io/crates/scm-record
[Docs]: https://img.shields.io/docsrs/scm-record
[link-docs]: https://docs.rs/scm-record/latest/scm_record/
[License]: https://img.shields.io/crates/l/scm-record
[link-license]: https://github.com/arxanas/scm-record/tree/main/scm-record

[![Build status]][link-build-status] [![Latest version]][link-latest-version] [![Docs]][link-docs] [![License]][link-license]

`scm-record` is a Rust library providing a terminal UI component for interactively selecting changes to include in a commit. It's designed to be embedded in source control tooling.

## Overview

Think of this as an interactive replacement for `git add -p`, or a reimplementation of `hg crecord`/`hg commit -i`. Given a set of changes made by the user, this component presents them in a navigable interface where users can:

- Navigate through files, functions, and individual changes
- Select/deselect changes at any granularity (files, sections, or individual lines)
- Review changes with syntax highlighting
- Navigate by semantic structure (when using the `tree-sitter` feature)

### Key Features

- **Semantic Navigation**: With the `tree-sitter` feature enabled, navigate by code structure (functions, classes, methods) rather than just diff hunks
- **Flexible Selection**: Toggle individual lines, entire sections, or whole files
- **Keyboard-Driven**: Vim-like keybindings for efficient navigation and selection
- **Mouse Support**: Click to focus, scroll to navigate
- **Embeddable**: Designed as a library component for integration into larger tools

## Integration

The `scm-record` library is directly integrated into these projects:

- [git-branchless](https://github.com/arxanas/git-branchless): the `git record -i` command lets you interactively select and commit changes
- [Jujutsu](https://github.com/martinvonz/jj): as the built-in diff editor; see the [`ui.diff-editor`](https://martinvonz.github.io/jj/latest/config/#editing-diffs) configuration option

## Standalone Executable

`syntax-diff-editor` is a standalone executable that uses `scm-record` as its UI. It can be used as a general-purpose diff/merge tool with any source control system.

### About This Fork

**Important Note**: `syntax-diff-editor` represents a significant change from the original `scm-diff-editor` project. The original `scm-diff-editor` (and the underlying `scm-record` library) is an absolutely wonderful piece of work. The tree-based selection approach for navigating and selecting changes is ergonomically brilliant - it makes reviewing and staging changes feel natural and intuitive in a way that's hard to describe until you experience it.

This fork exists primarily as an experiment: I was so impressed with the elegance of the tree selection interface that I wanted to explore enhancing it with semantic/syntax-aware navigation via tree-sitter. The rename to `syntax-diff-editor` reflects this focus on syntax-driven navigation, allowing you to navigate your diffs by functions, classes, and methods rather than just by hunks.

All credit for the core design and implementation goes to the original authors. This is merely an exploration of "what if we could navigate by code structure too?" built on top of their excellent foundation.

**Original Project**: [arxanas/scm-record](https://github.com/arxanas/scm-record)

### Installation

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

`scm-record` provides a rich set of keyboard shortcuts for efficient navigation and selection. Key bindings include:

- **Navigation**: `j/k` (or arrow keys) for up/down, `h/l` for in/out of hierarchy
- **Selection**: `Space` to toggle, `Enter` to toggle and advance, `a` for select/deselect all
- **Commit**: `c` to accept changes, `e` to edit commit message, `q` to quit

For a complete list of keyboard shortcuts and navigation tips, see the [Keyboard Controls documentation](docs/keyboard-controls.md).

## Contributing

Contributions are welcome! Here are some areas where `scm-record` could be improved:

### Feature Wishlist

- Make keybindings easier to discover ([#25](https://github.com/arxanas/scm-record/issues/25))
- Support accessing the menu with the keyboard ([#44](https://github.com/arxanas/scm-record/issues/44))
- Edit one side of the diff in an editor ([#83](https://github.com/arxanas/scm-record/issues/83))
- Multi-way split UI to split a commit into more than 2 commits ([#73](https://github.com/arxanas/scm-record/issues/73))
- Full mergetool support with conflict resolution commands
- Commands to select ours/theirs for merge conflicts

### Potential Integrations

Projects that could benefit from `scm-record` integration:

- [Sapling](https://sapling-scm.com/)
- [Stacked Git](https://stacked-git.github.io/)
- [Pijul](https://pijul.org/)
- [gitoxide/ein](https://github.com/Byron/gitoxide)
- [gitui](https://github.com/extrawurst/gitui)
- [Game of Trees](https://gameoftrees.org/)

## Documentation

- [Keyboard Controls](docs/keyboard-controls.md) - Complete reference for all keyboard shortcuts
- [ADR: Tree-sitter Integration](docs/adrs/001-tree-sitter-integration.md) - Design decisions for semantic navigation

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
