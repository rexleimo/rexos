# Mechanical Edits (Workspace Sandbox)

Use `agent run` when you want a targeted change and prefer manual review/commit.

## Example

```bash
cd /path/to/repo
loopforge agent run --workspace . --prompt "Rename Foo to Bar across the codebase. Update imports and keep tests passing."
```

Good prompts for this pattern:

- “Apply this change across all files and run the formatter.”
- “Update deprecated API calls and add a small regression test.”
- “Migrate config format and keep backwards compatibility.”
