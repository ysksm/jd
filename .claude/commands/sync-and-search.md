Run a JIRA sync followed by a search.

This command helps test the full sync and search workflow.

Arguments:
- $ARGUMENTS: Optional search query (default: show recent issues)

Steps:

1. First, sync the data:
```bash
cargo run -p jira-db-cli -- sync
```

2. If a search query was provided, search for it:
```bash
cargo run -p jira-db-cli -- search "$ARGUMENTS"
```

3. If no query was provided, list recent issues:
```bash
cargo run -p jira-db-cli -- search --limit 10
```

Report the results of both operations.
