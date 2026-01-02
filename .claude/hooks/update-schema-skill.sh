#!/bin/bash
# Update Schema Skill Hook: スキーマ変更時にdatabase-schema.mdを更新提案
# 実行タイミング: Stopフック時（スキーマファイルが変更された場合）

set -e
cd /home/user/jira-db

SCHEMA_FILE="crates/jira-db-core/src/infrastructure/database/schema.rs"
EMBEDDINGS_FILE="crates/jira-db-core/src/infrastructure/database/repositories/embeddings_repository.rs"
SKILL_FILE=".claude/skills/database-schema.md"

# Check if schema files were modified in this session
SCHEMA_MODIFIED=false

# Check git diff for schema changes
if git diff --name-only 2>/dev/null | grep -q "schema.rs\|embeddings_repository.rs"; then
    SCHEMA_MODIFIED=true
fi

# Also check staged changes
if git diff --cached --name-only 2>/dev/null | grep -q "schema.rs\|embeddings_repository.rs"; then
    SCHEMA_MODIFIED=true
fi

if [ "$SCHEMA_MODIFIED" = true ]; then
    echo ""
    echo "⚠️  Database Schema Changed!"
    echo "   Modified files:"
    git diff --name-only 2>/dev/null | grep -E "schema.rs|embeddings_repository.rs" || true
    git diff --cached --name-only 2>/dev/null | grep -E "schema.rs|embeddings_repository.rs" || true
    echo ""
    echo "   📝 Please update the skill file: $SKILL_FILE"
    echo "   Run: claude 'Update .claude/skills/database-schema.md to reflect the schema changes'"
    echo ""
fi

exit 0
