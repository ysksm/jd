#!/bin/bash
# Stop Hook: コード品質チェックとGitステータス確認
# 実行タイミング: エージェント応答完了時

set -e
cd /home/user/jira-db

echo "=== Post-Implementation Checks ==="

# 1. Cargo clippy (警告のみ表示、エラーで停止しない)
echo ""
echo "📋 Running cargo clippy..."
if cargo clippy --all-targets --all-features --message-format=short 2>&1 | grep -E "^(warning|error)" | head -10; then
    echo "   (showing first 10 warnings/errors)"
else
    echo "   ✅ No clippy warnings"
fi

# 2. Cargo test (簡易テスト実行)
echo ""
echo "🧪 Running cargo test..."
if cargo test --lib --quiet 2>&1 | tail -5; then
    echo "   ✅ Tests completed"
else
    echo "   ⚠️  Some tests may have failed"
fi

# 3. Git status check
echo ""
echo "📁 Git Status:"
if [[ -n $(git status --porcelain 2>/dev/null) ]]; then
    git status --short
    echo ""
    echo "   💡 Uncommitted changes detected"
else
    echo "   ✅ Working directory clean"
fi

# 4. Current branch info
echo ""
echo "🌿 Current Branch: $(git branch --show-current 2>/dev/null || echo 'N/A')"

exit 0
