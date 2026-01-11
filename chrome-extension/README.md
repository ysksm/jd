# JIRA DB Chrome Extension

Chrome extension for syncing JIRA data to a local DuckDB database (WASM). Provides offline search and analysis of JIRA issues.

## Features

- **Sync JIRA Issues**: Fetch and store JIRA issues locally using DuckDB WASM
- **Resume Support**: Sync can be interrupted and resumed from the last checkpoint
- **Incremental Sync**: Only fetch issues updated since the last sync
- **Full-text Search**: Search issues by summary, description, or key
- **Filters**: Filter by project, status, and assignee
- **Issue Details**: View issue details including change history
- **Navigate to JIRA**: Open issues directly in JIRA

## Installation

### Development Setup

1. Install dependencies:
   ```bash
   cd chrome-extension
   npm install
   ```

2. Generate icons:
   ```bash
   npm install canvas  # Optional dependency for icon generation
   node scripts/generate-icons.js
   ```

   Or manually create icon16.png, icon48.png, icon128.png in `public/icons/`

3. Build the extension:
   ```bash
   npm run build
   ```

4. Load in Chrome:
   - Open `chrome://extensions/`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select the `chrome-extension` directory

### Production Build

```bash
npm run build
```

Then zip the directory (excluding node_modules, src, and development files) for Chrome Web Store submission.

## Configuration

1. Click the extension icon and select the settings button (gear icon)
2. Enter your JIRA connection details:
   - **Endpoint**: Your JIRA URL (e.g., `https://your-domain.atlassian.net`)
   - **Username**: Your email address
   - **API Token**: Generate at [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens)
3. Click "Test Connection" to verify
4. Fetch projects and enable the ones you want to sync
5. Configure sync settings (incremental sync, batch size)

## Usage

### Syncing Data

1. Open the extension popup
2. Click the sync button (refresh icon)
3. Wait for sync to complete (progress shown)
4. If interrupted, sync will resume from the last checkpoint

### Searching Issues

1. Type in the search box to search by keyword
2. Use the project dropdown to filter by project
3. Use the status dropdown to filter by status
4. Click an issue to view details
5. Click "Open in JIRA" to navigate to the issue

## Architecture

```
chrome-extension/
├── manifest.json           # Chrome extension manifest (V3)
├── popup.html              # Popup UI
├── options.html            # Settings page
├── src/
│   ├── background/         # Service worker
│   │   └── index.ts        # Message handling, sync orchestration
│   ├── popup/
│   │   └── popup.ts        # Popup UI logic
│   ├── options/
│   │   └── options.ts      # Settings page logic
│   ├── lib/
│   │   ├── types.ts        # TypeScript types
│   │   ├── database.ts     # DuckDB WASM wrapper
│   │   ├── jira-client.ts  # JIRA API client
│   │   ├── settings.ts     # Chrome storage wrapper
│   │   └── sync-service.ts # Sync logic with checkpoints
│   └── styles/
│       ├── popup.css       # Popup styles
│       └── options.css     # Options page styles
├── public/
│   └── icons/              # Extension icons
├── dist/                   # Built files
├── build.js                # esbuild build script
├── package.json
└── tsconfig.json
```

## Key Technologies

- **DuckDB WASM**: In-browser SQL database for storing issues
- **Chrome Extension Manifest V3**: Modern extension architecture
- **TypeScript**: Type-safe development
- **esbuild**: Fast bundling

## Sync Features

### Resume Support (Checkpoints)

Sync progress is saved to Chrome storage after each batch:
- `lastProcessedUpdatedAt`: Timestamp of last processed issue
- `startPosition`: Number of issues processed
- `totalIssues`: Total issues to sync

If sync is interrupted (browser closed, network error), it resumes from the checkpoint.

### Incremental Sync

When enabled (default), only fetches issues updated since the last sync:
- Applies a configurable safety margin (default: 5 minutes)
- Falls back to full sync if no previous sync exists

## Permissions

- `storage`: Save settings and sync checkpoints
- `alarms`: Schedule background syncs (future)
- `host_permissions` for `*.atlassian.net`: Access JIRA API

## Future Enhancements

- [ ] Automatic background sync
- [ ] Export to CSV/Excel
- [ ] Integration with Claude Code web version
- [ ] Semantic search with embeddings
- [ ] Offline-first with service workers

## Troubleshooting

### Connection Failed

1. Verify your JIRA endpoint URL is correct
2. Check that your API token is valid
3. Ensure you have permission to access the JIRA projects

### Sync Stops/Hangs

1. Check your network connection
2. Close and reopen the popup
3. The sync will resume from the last checkpoint

### No Issues Shown

1. Verify at least one project is enabled
2. Try running a sync first
3. Check the console for errors (right-click extension icon → "Inspect popup")
