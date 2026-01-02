Build the jira-db project.

Run the following command:
```bash
cargo build
```

If building for release:
```bash
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH" && cargo build --release
```

After building, verify the build was successful by checking for any errors in the output.
