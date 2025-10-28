# ADR 002: Auto-Detect Directory Diff Mode

**Status**: Accepted

**Date**: 2025-10-28

**Authors**: Jake Swenson

## Context

`syntax-diff-editor` supports two modes of operation:
- **File mode**: Compare two individual files
- **Directory mode**: Recursively compare all files in two directories

Currently, directory mode requires users to explicitly pass the `--dir-diff` (or `-d`) flag. If users forget this flag when comparing directories, they receive an error because the tool attempts to read the directory as if it were a file.

This creates a friction point in user experience:
- Users must remember an additional flag
- The error message when forgetting the flag can be confusing
- The tool's behavior doesn't match user intent based on the arguments they provide

Most similar tools (like `diff`, `rsync`, etc.) detect whether arguments are files or directories automatically and adjust their behavior accordingly.

## Decision

**We will auto-detect directory diff mode when both paths are directories.**

### Proposed Behavior

1. **Both paths are directories** → Automatically enable directory mode (even without `--dir-diff` flag)
2. **Both paths are files** → Use file mode (current behavior)
3. **Mixed types** (one directory, one file) → Return a clear error message
4. **Explicit `--dir-diff` flag** → Always honored, regardless of path types (for backward compatibility and edge cases)
5. **Non-existent paths** → Continue to use existing error handling

### Implementation

The auto-detection will occur in the `process_opts()` function by:
1. Checking `Path::is_dir()` for both `left` and `right` paths
2. Setting `effective_dir_diff = opts.dir_diff || (left_is_dir && right_is_dir)`
3. Using `effective_dir_diff` instead of `opts.dir_diff` in the match statement

## Consequences

### Positive

1. **Improved UX** - Users can simply run `syntax-diff-editor dir1/ dir2/` without remembering flags
2. **Intuitive behavior** - Tool behavior matches user intent based on arguments
3. **Fewer errors** - Eliminates common user mistake of forgetting the flag
4. **Discoverability** - New users can discover directory mode naturally
5. **Matches conventions** - Behavior aligns with common Unix tool patterns
6. **Backward compatible** - Explicit flag continues to work for scripts and edge cases

### Negative

1. **Silent behavior change** - Tool behavior depends on filesystem state rather than only explicit flags
2. **Potential ambiguity** - Edge cases with symlinks or special files might be less obvious
3. **Debugging** - Users might not realize directory mode was auto-detected (mitigated by logging in verbose mode)
4. **Race conditions** - Theoretical race between checking path type and using it (minimal practical impact)

### Neutral

1. **Mixed path types** - Now explicitly handled with a clear error message (better than current behavior)
2. **Documentation burden** - Need to document auto-detection behavior clearly

## Alternatives Considered

### Alternative 1: Keep explicit flag only
**Rejected** - Prioritizes explicitness over user convenience. The current UX friction outweighs the benefits of explicit behavior.

### Alternative 2: Auto-detect but remove the flag
**Rejected** - Loses backward compatibility and removes explicit control for edge cases and scripting.

### Alternative 3: Add a `--auto-detect` flag
**Rejected** - Adds complexity without benefit. Auto-detection should be the default behavior with the explicit flag as an override.

## Implementation Notes

- The `-d`/`--dir-diff` flag documentation will be updated to mention auto-detection
- Verbose logging will indicate when auto-detection triggers
- Error messages for mixed path types will suggest corrective actions
- README will be updated with examples showing both explicit and auto-detected usage

## References

- Original implementation: `syntax-diff-editor/src/lib.rs`
- Related tools with similar behavior: `diff -r`, `rsync`, `cp -r`
- Issue: UX friction with directory comparisons
