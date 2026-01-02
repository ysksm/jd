Run code quality checks for the jira-db project.

Run the following commands in parallel:

1. Quick compile check:
```bash
cargo check
```

2. Run linter:
```bash
cargo clippy
```

3. Check formatting:
```bash
cargo fmt --check
```

Report any issues found and suggest fixes.
