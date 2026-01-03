#!/bin/bash
# Documentation Check Hook: ドキュメント更新が必要かチェック
# 実行タイミング: Stopフック時

set -e
cd /home/user/jira-db

echo ""
echo "📚 Documentation Check"

# Check for significant code changes that might require documentation updates
DOC_UPDATE_NEEDED=false
DOC_FILES_TO_CHECK=""

# Get list of changed files (staged + unstaged)
CHANGED_FILES=$(git diff --name-only 2>/dev/null; git diff --cached --name-only 2>/dev/null)

if [ -z "$CHANGED_FILES" ]; then
    echo "   ✅ No changes detected"
    exit 0
fi

# Check for schema changes -> update database-schema.md, CLAUDE.md, README.md
if echo "$CHANGED_FILES" | grep -qE "schema\.rs|embeddings_repository\.rs"; then
    echo "   ⚠️  Database schema changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK .claude/skills/database-schema.md CLAUDE.md README.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for CLI command changes -> update README.md, CLAUDE.md, docs/FEATURE_MATRIX.md
if echo "$CHANGED_FILES" | grep -qE "jira-db-cli/src/cli/commands\.rs|jira-db-cli/src/cli/handlers\.rs"; then
    echo "   ⚠️  CLI commands changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK README.md CLAUDE.md docs/FEATURE_MATRIX.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for Tauri command changes -> update docs/FEATURE_MATRIX.md
if echo "$CHANGED_FILES" | grep -qE "jira-db-tauri/src-tauri/src/commands/"; then
    echo "   ⚠️  Tauri commands changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK docs/FEATURE_MATRIX.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for MCP tool changes -> update README.md, CLAUDE.md, docs/MCP.md, docs/FEATURE_MATRIX.md
if echo "$CHANGED_FILES" | grep -qE "jira-db-mcp/src/tools/"; then
    echo "   ⚠️  MCP tools changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK README.md CLAUDE.md docs/MCP.md docs/FEATURE_MATRIX.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for use case changes -> update CLAUDE.md, docs/ARCHITECTURE.md, docs/FEATURE_MATRIX.md
if echo "$CHANGED_FILES" | grep -qE "application/use_cases/"; then
    echo "   ⚠️  Use cases changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK CLAUDE.md docs/ARCHITECTURE.md docs/FEATURE_MATRIX.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for entity changes -> update CLAUDE.md, docs/ARCHITECTURE.md
if echo "$CHANGED_FILES" | grep -qE "domain/entities/"; then
    echo "   ⚠️  Domain entities changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK CLAUDE.md docs/ARCHITECTURE.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for repository changes -> update docs/ARCHITECTURE.md
if echo "$CHANGED_FILES" | grep -qE "domain/repositories/|infrastructure/database/repositories/"; then
    echo "   ⚠️  Repositories changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK docs/ARCHITECTURE.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for embeddings changes -> update docs/EMBEDDINGS.md
if echo "$CHANGED_FILES" | grep -qE "external/embeddings/"; then
    echo "   ⚠️  Embeddings providers changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK docs/EMBEDDINGS.md CLAUDE.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for config changes -> update README.md, CLAUDE.md
if echo "$CHANGED_FILES" | grep -qE "infrastructure/config/settings\.rs"; then
    echo "   ⚠️  Configuration changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK README.md CLAUDE.md"
    DOC_UPDATE_NEEDED=true
fi

# Check for Cargo.toml changes (dependencies) -> update README.md
if echo "$CHANGED_FILES" | grep -qE "Cargo\.toml"; then
    echo "   ⚠️  Dependencies may have changed"
    DOC_FILES_TO_CHECK="$DOC_FILES_TO_CHECK README.md CLAUDE.md"
    DOC_UPDATE_NEEDED=true
fi

if [ "$DOC_UPDATE_NEEDED" = true ]; then
    echo ""
    echo "   📝 Consider updating these documentation files:"
    # Remove duplicates and print
    echo "$DOC_FILES_TO_CHECK" | tr ' ' '\n' | sort -u | grep -v '^$' | sed 's/^/      - /'
    echo ""
    echo "   💡 Run: claude 'Update documentation to reflect the code changes'"
else
    echo "   ✅ No documentation updates needed"
fi

exit 0
