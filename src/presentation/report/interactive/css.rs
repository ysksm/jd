/// インタラクティブレポート用CSSスタイル
pub fn get_css() -> &'static str {
    r#"
        :root {
            --jira-blue: #0052CC;
            --jira-blue-light: #DEEBFF;
            --jira-blue-dark: #0747A6;
            --jira-green: #36B37E;
            --jira-green-light: #E3FCEF;
            --jira-yellow: #FFAB00;
            --jira-yellow-light: #FFFAE6;
            --jira-red: #FF5630;
            --jira-red-light: #FFEBE6;
            --jira-purple: #6554C0;
            --jira-purple-light: #EAE6FF;
            --jira-teal: #00B8D9;
            --jira-gray: #42526E;
            --jira-gray-light: #97A0AF;
            --jira-bg: #FAFBFC;
            --jira-border: #DFE1E6;
            --jira-white: #FFFFFF;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background-color: var(--jira-bg);
            color: var(--jira-gray);
            line-height: 1.5;
        }

        .app {
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }

        .header {
            background: linear-gradient(135deg, var(--jira-blue), var(--jira-blue-dark));
            color: white;
            padding: 16px 24px;
            position: sticky;
            top: 0;
            z-index: 100;
        }

        .header-content {
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .header h1 {
            font-size: 22px;
            font-weight: 600;
        }

        .header-info {
            display: flex;
            gap: 24px;
            font-size: 14px;
            opacity: 0.9;
        }

        /* タブナビゲーション */
        .tab-nav {
            background: var(--jira-white);
            border-bottom: 1px solid var(--jira-border);
            position: sticky;
            top: 56px;
            z-index: 95;
        }

        .tab-nav-container {
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            gap: 0;
            overflow-x: auto;
        }

        .tab-btn {
            padding: 14px 24px;
            border: none;
            background: none;
            font-size: 14px;
            font-weight: 500;
            color: var(--jira-gray-light);
            cursor: pointer;
            border-bottom: 3px solid transparent;
            transition: all 0.2s;
            white-space: nowrap;
        }

        .tab-btn:hover {
            color: var(--jira-gray);
            background: var(--jira-bg);
        }

        .tab-btn.active {
            color: var(--jira-blue);
            border-bottom-color: var(--jira-blue);
        }

        .tab-content {
            display: none;
        }

        .tab-content.active {
            display: block;
        }

        .filters-bar {
            background: var(--jira-white);
            border-bottom: 1px solid var(--jira-border);
            padding: 16px 24px;
            position: sticky;
            top: 103px;
            z-index: 90;
        }

        .filters-container {
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            flex-wrap: wrap;
            gap: 12px;
            align-items: flex-end;
        }

        .filter-group {
            display: flex;
            flex-direction: column;
            gap: 4px;
        }

        .filter-group label {
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            color: var(--jira-gray-light);
        }

        .filter-group select,
        .filter-group input {
            padding: 8px 12px;
            border: 1px solid var(--jira-border);
            border-radius: 4px;
            font-size: 14px;
            min-width: 140px;
            background: var(--jira-white);
        }

        .filter-group input[type="text"] {
            min-width: 200px;
        }

        .filter-group select:focus,
        .filter-group input:focus {
            outline: none;
            border-color: var(--jira-blue);
            box-shadow: 0 0 0 2px var(--jira-blue-light);
        }

        .filter-actions {
            display: flex;
            gap: 8px;
            margin-left: auto;
        }

        .btn {
            padding: 8px 16px;
            border: none;
            border-radius: 4px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
        }

        .btn-primary {
            background: var(--jira-blue);
            color: white;
        }

        .btn-primary:hover {
            background: var(--jira-blue-dark);
        }

        .btn-secondary {
            background: var(--jira-bg);
            color: var(--jira-gray);
            border: 1px solid var(--jira-border);
        }

        .btn-secondary:hover {
            background: var(--jira-border);
        }

        .btn-icon {
            width: 32px;
            height: 32px;
            border: 1px solid var(--jira-border);
            background: var(--jira-white);
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }

        .main-content {
            flex: 1;
            padding: 24px;
            max-width: 1600px;
            margin: 0 auto;
            width: 100%;
        }

        .dashboard {
            display: flex;
            flex-direction: column;
            gap: 24px;
        }

        .charts-row {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 16px;
        }

        .chart-card {
            background: var(--jira-white);
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            padding: 16px;
            cursor: pointer;
            transition: box-shadow 0.2s;
        }

        .chart-card:hover {
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        }

        .chart-card h3 {
            font-size: 14px;
            color: var(--jira-gray);
            margin-bottom: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .chart-card canvas {
            max-height: 200px;
        }

        /* 統計カード */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 16px;
            margin-bottom: 24px;
        }

        .stat-card {
            background: var(--jira-white);
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            padding: 20px;
            text-align: center;
        }

        .stat-value {
            font-size: 32px;
            font-weight: 600;
            color: var(--jira-blue);
        }

        .stat-value.warning {
            color: var(--jira-yellow);
        }

        .stat-value.danger {
            color: var(--jira-red);
        }

        .stat-value.success {
            color: var(--jira-green);
        }

        .stat-label {
            font-size: 12px;
            color: var(--jira-gray-light);
            text-transform: uppercase;
            margin-top: 4px;
        }

        /* キャリーオーバーバッジ */
        .carryover-badge {
            display: inline-block;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 10px;
            font-weight: 600;
            background: var(--jira-yellow-light);
            color: #FF8B00;
            margin-left: 8px;
        }

        .issue-list-section {
            background: var(--jira-white);
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }

        .section-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 16px 20px;
            border-bottom: 1px solid var(--jira-border);
        }

        .section-header h2 {
            font-size: 18px;
            color: var(--jira-gray);
        }

        #filtered-count {
            font-size: 14px;
            font-weight: normal;
            color: var(--jira-gray-light);
        }

        .sort-controls {
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .sort-controls label {
            font-size: 13px;
            color: var(--jira-gray-light);
        }

        .sort-controls select {
            padding: 6px 10px;
            border: 1px solid var(--jira-border);
            border-radius: 4px;
            font-size: 13px;
        }

        .issue-table-container {
            overflow-x: auto;
        }

        .issue-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 14px;
        }

        .issue-table th {
            background: var(--jira-bg);
            text-align: left;
            padding: 12px 16px;
            font-weight: 600;
            color: var(--jira-gray);
            border-bottom: 1px solid var(--jira-border);
            cursor: pointer;
            user-select: none;
            white-space: nowrap;
        }

        .issue-table th:hover {
            background: var(--jira-border);
        }

        .issue-table td {
            padding: 12px 16px;
            border-bottom: 1px solid var(--jira-border);
        }

        .issue-table tr {
            cursor: pointer;
            transition: background 0.1s;
        }

        .issue-table tbody tr:hover {
            background: var(--jira-blue-light);
        }

        .issue-key {
            color: var(--jira-blue);
            font-weight: 500;
        }

        .issue-summary {
            max-width: 400px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .status-badge, .priority-badge {
            display: inline-block;
            padding: 3px 8px;
            border-radius: 3px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
        }

        .status-done { background: var(--jira-green-light); color: #006644; }
        .status-inprogress { background: var(--jira-blue-light); color: var(--jira-blue-dark); }
        .status-todo { background: #DFE1E6; color: var(--jira-gray); }
        .status-review { background: var(--jira-purple-light); color: #403294; }

        .priority-highest { background: var(--jira-red-light); color: #BF2600; }
        .priority-high { background: var(--jira-red-light); color: #DE350B; }
        .priority-medium { background: var(--jira-yellow-light); color: #FF8B00; }
        .priority-low { background: var(--jira-green-light); color: #006644; }
        .priority-lowest { background: #F4F5F7; color: var(--jira-gray); }

        .pagination {
            display: flex;
            justify-content: center;
            align-items: center;
            gap: 8px;
            padding: 16px;
            border-top: 1px solid var(--jira-border);
        }

        .pagination button {
            padding: 6px 12px;
            border: 1px solid var(--jira-border);
            background: var(--jira-white);
            border-radius: 4px;
            cursor: pointer;
            font-size: 13px;
        }

        .pagination button:hover:not(:disabled) {
            background: var(--jira-bg);
        }

        .pagination button:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        .pagination button.active {
            background: var(--jira-blue);
            color: white;
            border-color: var(--jira-blue);
        }

        .modal {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0,0,0,0.5);
            z-index: 1000;
            overflow-y: auto;
            padding: 24px;
        }

        .modal.active {
            display: flex;
            justify-content: center;
            align-items: flex-start;
        }

        .modal-content {
            background: var(--jira-white);
            border-radius: 8px;
            width: 100%;
            max-width: 800px;
            margin-top: 40px;
            box-shadow: 0 8px 32px rgba(0,0,0,0.2);
        }

        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 16px 24px;
            border-bottom: 1px solid var(--jira-border);
        }

        .modal-header h2 {
            color: var(--jira-blue);
            font-size: 18px;
        }

        .modal-close {
            width: 32px;
            height: 32px;
            border: none;
            background: none;
            font-size: 24px;
            cursor: pointer;
            color: var(--jira-gray-light);
            border-radius: 4px;
        }

        .modal-close:hover {
            background: var(--jira-bg);
            color: var(--jira-gray);
        }

        .modal-body {
            padding: 24px;
        }

        .issue-detail h3 {
            font-size: 20px;
            margin-bottom: 16px;
            color: var(--jira-gray);
        }

        .issue-meta {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 12px;
            margin-bottom: 24px;
        }

        .meta-item {
            display: flex;
            gap: 8px;
        }

        .meta-label {
            font-weight: 600;
            color: var(--jira-gray-light);
            font-size: 13px;
        }

        .change-history h4 {
            font-size: 16px;
            margin-bottom: 16px;
            color: var(--jira-gray);
        }

        .timeline {
            position: relative;
            padding-left: 24px;
            max-height: 400px;
            overflow-y: auto;
        }

        .timeline::before {
            content: '';
            position: absolute;
            left: 8px;
            top: 0;
            bottom: 0;
            width: 2px;
            background: var(--jira-border);
        }

        .timeline-item {
            position: relative;
            padding: 12px 0;
            padding-left: 16px;
            border-bottom: 1px solid var(--jira-border);
        }

        .timeline-item::before {
            content: '';
            position: absolute;
            left: -20px;
            top: 16px;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            background: var(--jira-blue);
            border: 2px solid var(--jira-white);
        }

        .timeline-date {
            font-size: 12px;
            color: var(--jira-gray-light);
            margin-bottom: 4px;
        }

        .timeline-field {
            font-weight: 600;
            color: var(--jira-gray);
        }

        .timeline-change {
            font-size: 13px;
            color: var(--jira-gray-light);
        }

        .timeline-author {
            font-size: 12px;
            color: var(--jira-gray-light);
            margin-top: 4px;
        }

        @media (max-width: 1024px) {
            .charts-row {
                grid-template-columns: repeat(2, 1fr);
            }
        }

        .burndown-section {
            margin-bottom: 8px;
        }

        .chart-card-wide {
            grid-column: 1 / -1;
        }

        .chart-card-wide canvas {
            max-height: 300px;
            width: 100%;
        }

        .chart-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 12px;
            flex-wrap: wrap;
            gap: 12px;
        }

        .chart-header h3 {
            margin-bottom: 0;
        }

        .chart-legend {
            display: flex;
            gap: 16px;
            flex-wrap: wrap;
        }

        .legend-item {
            display: flex;
            align-items: center;
            gap: 6px;
            font-size: 12px;
            color: var(--jira-gray);
        }

        .legend-color {
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 2px;
        }

        /* 日次進捗セクション */
        .daily-progress {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 24px;
            margin-bottom: 24px;
        }

        .progress-column {
            background: var(--jira-white);
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            padding: 16px;
        }

        .progress-column h3 {
            font-size: 14px;
            color: var(--jira-gray);
            margin-bottom: 16px;
            padding-bottom: 8px;
            border-bottom: 2px solid var(--jira-border);
        }

        .progress-column.yesterday h3 {
            border-bottom-color: var(--jira-green);
        }

        .progress-column.today h3 {
            border-bottom-color: var(--jira-blue);
        }

        .progress-column.blockers h3 {
            border-bottom-color: var(--jira-red);
        }

        .progress-list {
            list-style: none;
        }

        .progress-list li {
            padding: 8px 0;
            border-bottom: 1px solid var(--jira-border);
            font-size: 13px;
        }

        .progress-list li:last-child {
            border-bottom: none;
        }

        /* バグ経過日数 */
        .age-badge {
            display: inline-block;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 10px;
            font-weight: 600;
            margin-left: 8px;
        }

        .age-new { background: var(--jira-green-light); color: #006644; }
        .age-moderate { background: var(--jira-yellow-light); color: #FF8B00; }
        .age-old { background: var(--jira-red-light); color: #BF2600; }

        @media (max-width: 768px) {
            .header-content {
                flex-direction: column;
                gap: 8px;
                text-align: center;
            }

            .tab-nav-container {
                padding: 0 16px;
            }

            .tab-btn {
                padding: 12px 16px;
                font-size: 13px;
            }

            .filters-container {
                flex-direction: column;
            }

            .filter-group {
                width: 100%;
            }

            .filter-group select,
            .filter-group input {
                width: 100%;
            }

            .filter-actions {
                width: 100%;
                margin-left: 0;
            }

            .charts-row {
                grid-template-columns: 1fr;
            }

            .daily-progress {
                grid-template-columns: 1fr;
            }

            .section-header {
                flex-direction: column;
                gap: 12px;
            }

            .issue-table {
                font-size: 12px;
            }

            .issue-summary {
                max-width: 150px;
            }

            .chart-header {
                flex-direction: column;
                align-items: flex-start;
            }
        }
    "#
}
