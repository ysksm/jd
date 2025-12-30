/// インタラクティブレポート用HTMLテンプレート
pub fn get_html_template() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>JIRA Interactive Report - {date}</title>
    <style>
{css}
    </style>
</head>
<body>
    <div class="app">
        <header class="header">
            <div class="header-content">
                <h1>JIRA Interactive Report</h1>
                <div class="header-info">
                    <span id="generated-at">Generated: {generated_at}</span>
                    <span id="total-issues">Total: {total_issues} issues</span>
                </div>
            </div>
        </header>

        <!-- タブナビゲーション -->
        <nav class="tab-nav">
            <div class="tab-nav-container">
                <button class="tab-btn active" data-tab="daily-scrum">デイリースクラム</button>
                <button class="tab-btn" data-tab="sprint-planning">スプリント計画</button>
                <button class="tab-btn" data-tab="sprint-board">スプリントボード</button>
                <button class="tab-btn" data-tab="bug-tracker">バグトラッカー</button>
                <button class="tab-btn" data-tab="retrospective">振り返り</button>
                <button class="tab-btn" data-tab="trends">トレンド</button>
            </div>
        </nav>

        <!-- フィルターバー -->
        <nav class="filters-bar">
            <div class="filters-container">
                <div class="filter-group">
                    <label for="project-filter">プロジェクト</label>
                    <select id="project-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="status-filter">ステータス</label>
                    <select id="status-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="priority-filter">優先度</label>
                    <select id="priority-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="assignee-filter">担当者</label>
                    <select id="assignee-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="type-filter">タイプ</label>
                    <select id="type-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="sprint-filter">スプリント</label>
                    <select id="sprint-filter">
                        <option value="">すべて</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="date-from">開始日</label>
                    <input type="date" id="date-from">
                </div>
                <div class="filter-group">
                    <label for="date-to">終了日</label>
                    <input type="date" id="date-to">
                </div>
                <div class="filter-group">
                    <label for="search-input">検索</label>
                    <input type="text" id="search-input" placeholder="チケットを検索...">
                </div>
                <div class="filter-actions">
                    <button id="clear-filters" class="btn btn-secondary">クリア</button>
                    <button id="export-csv" class="btn btn-primary">CSV出力</button>
                </div>
            </div>
        </nav>

        <main class="main-content">
            <p id="filtered-count" style="margin-bottom: 16px; color: #97A0AF;"></p>

            <!-- デイリースクラムタブ -->
            <div id="tab-daily-scrum" class="tab-content active">
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value success" id="daily-completed-count">0</div>
                        <div class="stat-label">昨日完了</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="daily-inprogress-count">0</div>
                        <div class="stat-label">進行中</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value danger" id="daily-blockers-count">0</div>
                        <div class="stat-label">ブロッカー</div>
                    </div>
                </div>

                <div class="daily-progress">
                    <div class="progress-column yesterday">
                        <h3>昨日完了したこと</h3>
                        <ul class="progress-list" id="yesterday-list"></ul>
                    </div>
                    <div class="progress-column today">
                        <h3>今日やること</h3>
                        <ul class="progress-list" id="today-list"></ul>
                    </div>
                    <div class="progress-column blockers">
                        <h3>ブロッカー・課題</h3>
                        <ul class="progress-list" id="blockers-list"></ul>
                    </div>
                </div>
            </div>

            <!-- スプリント計画タブ -->
            <div id="tab-sprint-planning" class="tab-content">
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value" id="backlog-total">0</div>
                        <div class="stat-label">バックログ総数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value danger" id="backlog-bugs">0</div>
                        <div class="stat-label">バグ</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value success" id="backlog-stories">0</div>
                        <div class="stat-label">ストーリー</div>
                    </div>
                </div>

                <div class="issue-list-section">
                    <div class="section-header">
                        <h2>バックログ（スプリント未設定）</h2>
                    </div>
                    <div class="issue-table-container">
                        <table class="issue-table">
                            <thead>
                                <tr>
                                    <th>Key</th>
                                    <th>概要</th>
                                    <th>優先度</th>
                                    <th>タイプ</th>
                                    <th>担当者</th>
                                    <th>作成日</th>
                                </tr>
                            </thead>
                            <tbody id="backlog-tbody"></tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- スプリントボードタブ -->
            <div id="tab-sprint-board" class="tab-content">
                <h2 style="margin-bottom: 16px;">現在のスプリント: <span id="current-sprint-name">-</span></h2>

                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value" id="sprint-total">0</div>
                        <div class="stat-label">総チケット数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value success" id="sprint-done">0</div>
                        <div class="stat-label">完了</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="sprint-inprogress">0</div>
                        <div class="stat-label">進行中</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="sprint-todo">0</div>
                        <div class="stat-label">未着手</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value warning" id="sprint-carryover">0</div>
                        <div class="stat-label">キャリーオーバー</div>
                    </div>
                </div>

                <div class="chart-card chart-card-wide" style="margin-bottom: 24px;">
                    <div class="chart-header">
                        <h3>スプリントバーンダウン</h3>
                        <div class="chart-legend">
                            <span class="legend-item"><span class="legend-color" style="background:#DFE1E6"></span>理想線</span>
                            <span class="legend-item"><span class="legend-color" style="background:#FF5630"></span>実績</span>
                        </div>
                    </div>
                    <canvas id="sprint-burndown-chart"></canvas>
                </div>

                <div class="issue-list-section">
                    <div class="section-header">
                        <h2>スプリントチケット</h2>
                    </div>
                    <div class="issue-table-container">
                        <table class="issue-table">
                            <thead>
                                <tr>
                                    <th>Key</th>
                                    <th>概要</th>
                                    <th>ステータス</th>
                                    <th>優先度</th>
                                    <th>担当者</th>
                                </tr>
                            </thead>
                            <tbody id="sprint-tbody"></tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- バグトラッカータブ -->
            <div id="tab-bug-tracker" class="tab-content">
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value" id="bug-total">0</div>
                        <div class="stat-label">バグ総数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value danger" id="bug-open">0</div>
                        <div class="stat-label">未解決</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value danger" id="bug-critical">0</div>
                        <div class="stat-label">クリティカル</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value warning" id="bug-old">0</div>
                        <div class="stat-label">30日以上経過</div>
                    </div>
                </div>

                <div class="charts-row" style="margin-bottom: 24px;">
                    <div class="chart-card">
                        <h3>優先度別バグ</h3>
                        <canvas id="bug-priority-chart"></canvas>
                    </div>
                </div>

                <div class="issue-list-section">
                    <div class="section-header">
                        <h2>未解決バグ一覧</h2>
                    </div>
                    <div class="issue-table-container">
                        <table class="issue-table">
                            <thead>
                                <tr>
                                    <th>Key</th>
                                    <th>概要</th>
                                    <th>ステータス</th>
                                    <th>優先度</th>
                                    <th>担当者</th>
                                    <th>経過日数</th>
                                </tr>
                            </thead>
                            <tbody id="bug-tbody"></tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- 振り返りタブ -->
            <div id="tab-retrospective" class="tab-content">
                <h2 style="margin-bottom: 16px;">スプリント: <span id="retro-sprint-name">-</span></h2>

                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value success" id="retro-completed">0</div>
                        <div class="stat-label">完了チケット</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="retro-velocity">0</div>
                        <div class="stat-label">ベロシティ（チケット数）</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value danger" id="retro-bugs-found">0</div>
                        <div class="stat-label">発生バグ</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value warning" id="retro-carryover-next">0</div>
                        <div class="stat-label">次スプリントへ持越</div>
                    </div>
                </div>

                <div class="charts-row" style="margin-bottom: 24px;">
                    <div class="chart-card">
                        <h3>完了状況</h3>
                        <canvas id="retro-completion-chart"></canvas>
                    </div>
                </div>

                <div class="issue-list-section">
                    <div class="section-header">
                        <h2>担当者別パフォーマンス</h2>
                    </div>
                    <div class="issue-table-container">
                        <table class="issue-table">
                            <thead>
                                <tr>
                                    <th>担当者</th>
                                    <th>割当数</th>
                                    <th>完了数</th>
                                    <th>完了率</th>
                                </tr>
                            </thead>
                            <tbody id="retro-assignee-tbody"></tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- トレンドタブ -->
            <div id="tab-trends" class="tab-content">
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-value" id="trend-total-created">0</div>
                        <div class="stat-label">総作成数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value success" id="trend-total-resolved">0</div>
                        <div class="stat-label">総解決数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value" id="trend-avg-per-month">0</div>
                        <div class="stat-label">月平均作成数</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-value success" id="trend-resolution-rate">0%</div>
                        <div class="stat-label">解決率</div>
                    </div>
                </div>

                <div class="chart-card chart-card-wide" style="margin-bottom: 24px;">
                    <div class="chart-header">
                        <h3>月別チケット推移</h3>
                        <div class="chart-legend">
                            <span class="legend-item"><span class="legend-color" style="background:#0052CC"></span>作成</span>
                            <span class="legend-item"><span class="legend-color" style="background:#36B37E"></span>解決</span>
                        </div>
                    </div>
                    <canvas id="trends-chart"></canvas>
                </div>

                <div class="charts-row">
                    <div class="chart-card">
                        <h3>タイプ別分布</h3>
                        <canvas id="type-trends-chart"></canvas>
                    </div>
                </div>
            </div>
        </main>

        <!-- モーダル -->
        <div class="modal" id="issue-modal">
            <div class="modal-content">
                <div class="modal-header">
                    <h2 id="modal-issue-key"></h2>
                    <button class="modal-close" id="modal-close">&times;</button>
                </div>
                <div class="modal-body">
                    <div class="issue-detail">
                        <h3 id="modal-issue-summary"></h3>
                        <div class="issue-meta">
                            <div class="meta-item">
                                <span class="meta-label">ステータス:</span>
                                <span id="modal-status" class="status-badge"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">優先度:</span>
                                <span id="modal-priority" class="priority-badge"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">担当者:</span>
                                <span id="modal-assignee"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">報告者:</span>
                                <span id="modal-reporter"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">タイプ:</span>
                                <span id="modal-type"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">スプリント:</span>
                                <span id="modal-sprint"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">コンポーネント:</span>
                                <span id="modal-components"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">ラベル:</span>
                                <span id="modal-labels"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">作成日:</span>
                                <span id="modal-created"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">更新日:</span>
                                <span id="modal-updated"></span>
                            </div>
                        </div>
                    </div>
                    <div class="change-history">
                        <h4>変更履歴</h4>
                        <div id="history-timeline" class="timeline"></div>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <script>
const REPORT_DATA = {json_data};

{js}
    </script>
</body>
</html>
"#
}
