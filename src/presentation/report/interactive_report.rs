use crate::application::use_cases::ReportData;

pub fn generate_interactive_report(data: &ReportData) -> String {
    let json_data = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());

    format!(r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>JIRA Interactive Report - {}</title>
    <style>
{}
    </style>
</head>
<body>
    <div class="app">
        <header class="header">
            <div class="header-content">
                <h1>JIRA Interactive Report</h1>
                <div class="header-info">
                    <span id="generated-at">Generated: {}</span>
                    <span id="total-issues">Total: {} issues</span>
                </div>
            </div>
        </header>

        <nav class="filters-bar">
            <div class="filters-container">
                <div class="filter-group">
                    <label for="project-filter">Project</label>
                    <select id="project-filter">
                        <option value="">All Projects</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="status-filter">Status</label>
                    <select id="status-filter">
                        <option value="">All Statuses</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="priority-filter">Priority</label>
                    <select id="priority-filter">
                        <option value="">All Priorities</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="assignee-filter">Assignee</label>
                    <select id="assignee-filter">
                        <option value="">All Assignees</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="type-filter">Type</label>
                    <select id="type-filter">
                        <option value="">All Types</option>
                    </select>
                </div>
                <div class="filter-group">
                    <label for="date-from">From</label>
                    <input type="date" id="date-from">
                </div>
                <div class="filter-group">
                    <label for="date-to">To</label>
                    <input type="date" id="date-to">
                </div>
                <div class="filter-group">
                    <label for="search-input">Search</label>
                    <input type="text" id="search-input" placeholder="Search issues...">
                </div>
                <div class="filter-actions">
                    <button id="clear-filters" class="btn btn-secondary">Clear</button>
                    <button id="export-csv" class="btn btn-primary">Export CSV</button>
                </div>
            </div>
        </nav>

        <main class="main-content">
            <div class="dashboard">
                <div class="burndown-section">
                    <div class="chart-card chart-card-wide" id="burndown-chart-card">
                        <div class="chart-header">
                            <h3>Issue Timeline (Burndown)</h3>
                            <div class="chart-legend">
                                <span class="legend-item"><span class="legend-color" style="background:#0052CC"></span>Total Created</span>
                                <span class="legend-item"><span class="legend-color" style="background:#36B37E"></span>Resolved</span>
                                <span class="legend-item"><span class="legend-color" style="background:#FF5630"></span>Active (Open)</span>
                            </div>
                        </div>
                        <canvas id="burndown-chart"></canvas>
                    </div>
                </div>

                <div class="charts-row">
                    <div class="chart-card" id="status-chart-card">
                        <h3>Status Distribution</h3>
                        <canvas id="status-chart"></canvas>
                    </div>
                    <div class="chart-card" id="priority-chart-card">
                        <h3>Priority Distribution</h3>
                        <canvas id="priority-chart"></canvas>
                    </div>
                    <div class="chart-card" id="type-chart-card">
                        <h3>Issue Type Distribution</h3>
                        <canvas id="type-chart"></canvas>
                    </div>
                    <div class="chart-card" id="assignee-chart-card">
                        <h3>Top Assignees</h3>
                        <canvas id="assignee-chart"></canvas>
                    </div>
                </div>

                <div class="issue-list-section">
                    <div class="section-header">
                        <h2>Issues <span id="filtered-count"></span></h2>
                        <div class="sort-controls">
                            <label for="sort-by">Sort by:</label>
                            <select id="sort-by">
                                <option value="key">Key</option>
                                <option value="summary">Summary</option>
                                <option value="status">Status</option>
                                <option value="priority">Priority</option>
                                <option value="assignee">Assignee</option>
                                <option value="created" selected>Created</option>
                                <option value="updated">Updated</option>
                            </select>
                            <button id="sort-order" class="btn-icon" title="Toggle sort order">&#x2195;</button>
                        </div>
                    </div>
                    <div class="issue-table-container">
                        <table class="issue-table" id="issue-table">
                            <thead>
                                <tr>
                                    <th data-sort="key">Key</th>
                                    <th data-sort="summary">Summary</th>
                                    <th data-sort="status">Status</th>
                                    <th data-sort="priority">Priority</th>
                                    <th data-sort="assignee">Assignee</th>
                                    <th data-sort="issue_type">Type</th>
                                    <th data-sort="created">Created</th>
                                </tr>
                            </thead>
                            <tbody id="issue-tbody"></tbody>
                        </table>
                    </div>
                    <div class="pagination" id="pagination"></div>
                </div>
            </div>
        </main>

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
                                <span class="meta-label">Status:</span>
                                <span id="modal-status" class="status-badge"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Priority:</span>
                                <span id="modal-priority" class="priority-badge"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Assignee:</span>
                                <span id="modal-assignee"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Reporter:</span>
                                <span id="modal-reporter"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Type:</span>
                                <span id="modal-type"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Components:</span>
                                <span id="modal-components"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Labels:</span>
                                <span id="modal-labels"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Created:</span>
                                <span id="modal-created"></span>
                            </div>
                            <div class="meta-item">
                                <span class="meta-label">Updated:</span>
                                <span id="modal-updated"></span>
                            </div>
                        </div>
                    </div>
                    <div class="change-history">
                        <h4>Change History</h4>
                        <div id="history-timeline" class="timeline"></div>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <script>
const REPORT_DATA = {};

{}
    </script>
</body>
</html>
"#,
        data.generated_at.format("%Y-%m-%d"),
        get_interactive_css(),
        data.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
        data.total_issues,
        json_data,
        get_interactive_js()
    )
}

fn get_interactive_css() -> &'static str {
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

        .filters-bar {
            background: var(--jira-white);
            border-bottom: 1px solid var(--jira-border);
            padding: 16px 24px;
            position: sticky;
            top: 56px;
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

        @media (max-width: 768px) {
            .header-content {
                flex-direction: column;
                gap: 8px;
                text-align: center;
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

fn get_interactive_js() -> &'static str {
    r#"
(function() {
    'use strict';

    // State
    let allIssues = [];
    let filteredIssues = [];
    let currentPage = 1;
    const pageSize = 25;
    let sortBy = 'created';
    let sortOrder = 'desc';
    let activeChartFilter = null;

    // Initialize
    document.addEventListener('DOMContentLoaded', init);

    function init() {
        parseData();
        populateFilters();
        bindEvents();
        applyFilters();
        renderCharts();
    }

    function parseData() {
        allIssues = [];
        REPORT_DATA.projects.forEach(project => {
            project.issues.forEach(issue => {
                allIssues.push({
                    ...issue,
                    project_key: project.key,
                    project_name: project.name
                });
            });
        });
    }

    function populateFilters() {
        const projects = new Set();
        const statuses = new Set();
        const priorities = new Set();
        const assignees = new Set();
        const types = new Set();

        allIssues.forEach(issue => {
            projects.add(issue.project_key);
            statuses.add(issue.status);
            priorities.add(issue.priority);
            assignees.add(issue.assignee);
            types.add(issue.issue_type);
        });

        populateSelect('project-filter', Array.from(projects).sort());
        populateSelect('status-filter', Array.from(statuses).sort());
        populateSelect('priority-filter', Array.from(priorities).sort());
        populateSelect('assignee-filter', Array.from(assignees).sort());
        populateSelect('type-filter', Array.from(types).sort());
    }

    function populateSelect(id, options) {
        const select = document.getElementById(id);
        const firstOption = select.options[0];
        select.innerHTML = '';
        select.appendChild(firstOption);
        options.forEach(opt => {
            const option = document.createElement('option');
            option.value = opt;
            option.textContent = opt;
            select.appendChild(option);
        });
    }

    function bindEvents() {
        // Filter events
        ['project-filter', 'status-filter', 'priority-filter', 'assignee-filter', 'type-filter'].forEach(id => {
            document.getElementById(id).addEventListener('change', () => {
                activeChartFilter = null;
                applyFilters();
                renderCharts();
            });
        });

        document.getElementById('date-from').addEventListener('change', () => { applyFilters(); });
        document.getElementById('date-to').addEventListener('change', () => { applyFilters(); });

        document.getElementById('search-input').addEventListener('input', debounce(() => {
            applyFilters();
        }, 300));

        document.getElementById('clear-filters').addEventListener('click', clearFilters);
        document.getElementById('export-csv').addEventListener('click', exportCsv);

        // Sort events
        document.getElementById('sort-by').addEventListener('change', (e) => {
            sortBy = e.target.value;
            applyFilters();
        });

        document.getElementById('sort-order').addEventListener('click', () => {
            sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
            applyFilters();
        });

        // Table header sort
        document.querySelectorAll('.issue-table th[data-sort]').forEach(th => {
            th.addEventListener('click', () => {
                sortBy = th.dataset.sort;
                sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
                document.getElementById('sort-by').value = sortBy;
                applyFilters();
            });
        });

        // Modal events
        document.getElementById('modal-close').addEventListener('click', closeModal);
        document.getElementById('issue-modal').addEventListener('click', (e) => {
            if (e.target.id === 'issue-modal') closeModal();
        });
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') closeModal();
        });

        // Chart click events
        ['status', 'priority', 'type', 'assignee'].forEach(chartType => {
            document.getElementById(`${chartType}-chart-card`).addEventListener('click', (e) => {
                handleChartClick(e, chartType);
            });
        });
    }

    function applyFilters() {
        const projectFilter = document.getElementById('project-filter').value;
        const statusFilter = document.getElementById('status-filter').value;
        const priorityFilter = document.getElementById('priority-filter').value;
        const assigneeFilter = document.getElementById('assignee-filter').value;
        const typeFilter = document.getElementById('type-filter').value;
        const dateFrom = document.getElementById('date-from').value;
        const dateTo = document.getElementById('date-to').value;
        const searchQuery = document.getElementById('search-input').value.toLowerCase();

        filteredIssues = allIssues.filter(issue => {
            if (projectFilter && issue.project_key !== projectFilter) return false;
            if (statusFilter && issue.status !== statusFilter) return false;
            if (priorityFilter && issue.priority !== priorityFilter) return false;
            if (assigneeFilter && issue.assignee !== assigneeFilter) return false;
            if (typeFilter && issue.issue_type !== typeFilter) return false;

            if (dateFrom && issue.created_date) {
                const createdDate = new Date(issue.created_date).toISOString().split('T')[0];
                if (createdDate < dateFrom) return false;
            }

            if (dateTo && issue.created_date) {
                const createdDate = new Date(issue.created_date).toISOString().split('T')[0];
                if (createdDate > dateTo) return false;
            }

            if (searchQuery) {
                const searchTarget = `${issue.key} ${issue.summary} ${issue.assignee} ${issue.reporter}`.toLowerCase();
                if (!searchTarget.includes(searchQuery)) return false;
            }

            return true;
        });

        // Apply chart filter if active
        if (activeChartFilter) {
            filteredIssues = filteredIssues.filter(issue => {
                return issue[activeChartFilter.field] === activeChartFilter.value;
            });
        }

        // Sort
        filteredIssues.sort((a, b) => {
            let aVal = a[sortBy] || '';
            let bVal = b[sortBy] || '';

            if (sortBy === 'created' || sortBy === 'updated') {
                aVal = a[sortBy + '_date'] ? new Date(a[sortBy + '_date']) : new Date(0);
                bVal = b[sortBy + '_date'] ? new Date(b[sortBy + '_date']) : new Date(0);
            }

            if (typeof aVal === 'string') aVal = aVal.toLowerCase();
            if (typeof bVal === 'string') bVal = bVal.toLowerCase();

            if (aVal < bVal) return sortOrder === 'asc' ? -1 : 1;
            if (aVal > bVal) return sortOrder === 'asc' ? 1 : -1;
            return 0;
        });

        currentPage = 1;
        renderIssueTable();
        updateFilteredCount();
    }

    function renderIssueTable() {
        const tbody = document.getElementById('issue-tbody');
        const start = (currentPage - 1) * pageSize;
        const end = start + pageSize;
        const pageIssues = filteredIssues.slice(start, end);

        tbody.innerHTML = pageIssues.map(issue => `
            <tr data-key="${issue.key}">
                <td class="issue-key">${issue.key}</td>
                <td class="issue-summary" title="${escapeHtml(issue.summary)}">${escapeHtml(issue.summary)}</td>
                <td><span class="status-badge ${getStatusClass(issue.status)}">${issue.status}</span></td>
                <td><span class="priority-badge ${getPriorityClass(issue.priority)}">${issue.priority}</span></td>
                <td>${issue.assignee}</td>
                <td>${issue.issue_type}</td>
                <td>${formatDate(issue.created_date)}</td>
            </tr>
        `).join('');

        // Bind row click events
        tbody.querySelectorAll('tr').forEach(tr => {
            tr.addEventListener('click', () => {
                const key = tr.dataset.key;
                const issue = allIssues.find(i => i.key === key);
                if (issue) openModal(issue);
            });
        });

        renderPagination();
    }

    function renderPagination() {
        const totalPages = Math.ceil(filteredIssues.length / pageSize);
        const pagination = document.getElementById('pagination');

        if (totalPages <= 1) {
            pagination.innerHTML = '';
            return;
        }

        let html = '';
        html += `<button ${currentPage === 1 ? 'disabled' : ''} onclick="window.goToPage(${currentPage - 1})">Prev</button>`;

        const maxButtons = 7;
        let startPage = Math.max(1, currentPage - 3);
        let endPage = Math.min(totalPages, startPage + maxButtons - 1);
        startPage = Math.max(1, endPage - maxButtons + 1);

        if (startPage > 1) {
            html += `<button onclick="window.goToPage(1)">1</button>`;
            if (startPage > 2) html += `<span>...</span>`;
        }

        for (let i = startPage; i <= endPage; i++) {
            html += `<button class="${i === currentPage ? 'active' : ''}" onclick="window.goToPage(${i})">${i}</button>`;
        }

        if (endPage < totalPages) {
            if (endPage < totalPages - 1) html += `<span>...</span>`;
            html += `<button onclick="window.goToPage(${totalPages})">${totalPages}</button>`;
        }

        html += `<button ${currentPage === totalPages ? 'disabled' : ''} onclick="window.goToPage(${currentPage + 1})">Next</button>`;

        pagination.innerHTML = html;
    }

    window.goToPage = function(page) {
        currentPage = page;
        renderIssueTable();
    };

    function updateFilteredCount() {
        document.getElementById('filtered-count').textContent = `(${filteredIssues.length} of ${allIssues.length})`;
    }

    function renderCharts() {
        renderPieChart('status-chart', getDistribution('status'));
        renderPieChart('priority-chart', getDistribution('priority'));
        renderPieChart('type-chart', getDistribution('issue_type'));
        renderBarChart('assignee-chart', getDistribution('assignee', 10));
        renderBurndownChart();
    }

    function renderBurndownChart() {
        const canvas = document.getElementById('burndown-chart');
        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 600;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // Get timeline data from the first project (or aggregate all)
        let timelineData = [];
        if (REPORT_DATA.projects && REPORT_DATA.projects.length > 0) {
            // If single project selected, use that project's timeline
            const projectFilter = document.getElementById('project-filter').value;
            if (projectFilter) {
                const project = REPORT_DATA.projects.find(p => p.key === projectFilter);
                if (project && project.timeline_data) {
                    timelineData = project.timeline_data;
                }
            } else {
                // Aggregate timeline data from all projects
                timelineData = aggregateTimelineData();
            }
        }

        if (timelineData.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('No timeline data available', canvas.width / 2, canvas.height / 2);
            return;
        }

        // Chart dimensions
        const padding = { top: 40, right: 40, bottom: 60, left: 70 };
        const chartWidth = canvas.width - padding.left - padding.right;
        const chartHeight = canvas.height - padding.top - padding.bottom;

        // Find max values for scaling
        const maxCreated = Math.max(...timelineData.map(d => d.created));
        const maxResolved = Math.max(...timelineData.map(d => d.resolved));
        const maxActive = Math.max(...timelineData.map(d => d.active));
        const maxValue = Math.max(maxCreated, maxResolved, maxActive, 1);

        // Scale functions
        const scaleX = (i) => padding.left + (i / (timelineData.length - 1 || 1)) * chartWidth;
        const scaleY = (val) => padding.top + chartHeight - (val / maxValue) * chartHeight;

        // Draw grid lines
        ctx.strokeStyle = '#DFE1E6';
        ctx.lineWidth = 1;
        const gridLines = 5;
        for (let i = 0; i <= gridLines; i++) {
            const y = padding.top + (i / gridLines) * chartHeight;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(padding.left + chartWidth, y);
            ctx.stroke();

            // Y-axis labels
            const value = Math.round(maxValue * (1 - i / gridLines));
            ctx.fillStyle = '#97A0AF';
            ctx.font = '11px sans-serif';
            ctx.textAlign = 'right';
            ctx.fillText(value.toString(), padding.left - 10, y + 4);
        }

        // Draw X-axis labels (show selected dates)
        ctx.fillStyle = '#97A0AF';
        ctx.font = '10px sans-serif';
        ctx.textAlign = 'center';
        const labelStep = Math.max(1, Math.floor(timelineData.length / 10));
        timelineData.forEach((point, i) => {
            if (i % labelStep === 0 || i === timelineData.length - 1) {
                const x = scaleX(i);
                ctx.save();
                ctx.translate(x, padding.top + chartHeight + 15);
                ctx.rotate(-Math.PI / 6);
                ctx.fillText(point.date, 0, 0);
                ctx.restore();
            }
        });

        // Draw lines - Total Created (blue)
        ctx.beginPath();
        ctx.strokeStyle = '#0052CC';
        ctx.lineWidth = 2.5;
        timelineData.forEach((point, i) => {
            const x = scaleX(i);
            const y = scaleY(point.created);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
        ctx.stroke();

        // Draw lines - Resolved (green)
        ctx.beginPath();
        ctx.strokeStyle = '#36B37E';
        ctx.lineWidth = 2.5;
        timelineData.forEach((point, i) => {
            const x = scaleX(i);
            const y = scaleY(point.resolved);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
        ctx.stroke();

        // Draw lines - Active/Open (red) - this is the burndown line
        ctx.beginPath();
        ctx.strokeStyle = '#FF5630';
        ctx.lineWidth = 3;
        timelineData.forEach((point, i) => {
            const x = scaleX(i);
            const y = scaleY(point.active);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
        ctx.stroke();

        // Fill area under active line for visual emphasis
        ctx.beginPath();
        ctx.fillStyle = 'rgba(255, 86, 48, 0.1)';
        ctx.moveTo(scaleX(0), scaleY(0));
        timelineData.forEach((point, i) => {
            ctx.lineTo(scaleX(i), scaleY(point.active));
        });
        ctx.lineTo(scaleX(timelineData.length - 1), scaleY(0));
        ctx.closePath();
        ctx.fill();

        // Draw points on lines
        const drawPoints = (data, field, color) => {
            const pointStep = Math.max(1, Math.floor(timelineData.length / 30));
            data.forEach((point, i) => {
                if (i % pointStep === 0 || i === data.length - 1) {
                    const x = scaleX(i);
                    const y = scaleY(point[field]);
                    ctx.beginPath();
                    ctx.fillStyle = color;
                    ctx.arc(x, y, 4, 0, Math.PI * 2);
                    ctx.fill();
                    ctx.beginPath();
                    ctx.fillStyle = '#FFFFFF';
                    ctx.arc(x, y, 2, 0, Math.PI * 2);
                    ctx.fill();
                }
            });
        };

        drawPoints(timelineData, 'created', '#0052CC');
        drawPoints(timelineData, 'resolved', '#36B37E');
        drawPoints(timelineData, 'active', '#FF5630');

        // Show latest values
        const latest = timelineData[timelineData.length - 1];
        ctx.font = 'bold 12px sans-serif';
        ctx.textAlign = 'left';
        const infoX = padding.left + 10;
        const infoY = padding.top + 20;
        ctx.fillStyle = '#0052CC';
        ctx.fillText(`Total Created: ${latest.created}`, infoX, infoY);
        ctx.fillStyle = '#36B37E';
        ctx.fillText(`Resolved: ${latest.resolved}`, infoX + 150, infoY);
        ctx.fillStyle = '#FF5630';
        ctx.fillText(`Active (Open): ${latest.active}`, infoX + 280, infoY);
    }

    function aggregateTimelineData() {
        // Aggregate timeline data from all projects
        const dateMap = new Map();

        REPORT_DATA.projects.forEach(project => {
            if (!project.timeline_data) return;
            project.timeline_data.forEach(point => {
                if (dateMap.has(point.date)) {
                    const existing = dateMap.get(point.date);
                    existing.created += point.created;
                    existing.resolved += point.resolved;
                    existing.active += point.active;
                } else {
                    dateMap.set(point.date, {
                        date: point.date,
                        created: point.created,
                        resolved: point.resolved,
                        active: point.active
                    });
                }
            });
        });

        // Sort by date and return
        return Array.from(dateMap.values()).sort((a, b) => a.date.localeCompare(b.date));
    }

    function getDistribution(field, limit = null) {
        const counts = {};
        filteredIssues.forEach(issue => {
            const value = issue[field] || 'Unknown';
            counts[value] = (counts[value] || 0) + 1;
        });

        let items = Object.entries(counts).map(([name, count]) => ({ name, count }));
        items.sort((a, b) => b.count - a.count);

        if (limit) items = items.slice(0, limit);
        return items;
    }

    function renderPieChart(canvasId, data) {
        const canvas = document.getElementById(canvasId);
        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 400;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        if (data.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('No data', canvas.width / 2, canvas.height / 2);
            return;
        }

        const total = data.reduce((sum, item) => sum + item.count, 0);
        const centerX = canvas.width / 4;
        const centerY = canvas.height / 2;
        const radius = Math.min(centerX, centerY) - 20;

        let startAngle = -Math.PI / 2;
        const colors = getChartColors(data.length);

        data.forEach((item, i) => {
            const sliceAngle = (item.count / total) * 2 * Math.PI;
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.arc(centerX, centerY, radius, startAngle, startAngle + sliceAngle);
            ctx.closePath();
            ctx.fillStyle = colors[i];
            ctx.fill();
            startAngle += sliceAngle;
        });

        // Legend
        const legendX = canvas.width / 2 + 20;
        let legendY = 30;
        ctx.font = '12px sans-serif';
        ctx.textAlign = 'left';

        data.forEach((item, i) => {
            ctx.fillStyle = colors[i];
            ctx.fillRect(legendX, legendY - 10, 12, 12);
            ctx.fillStyle = '#42526E';
            const percent = ((item.count / total) * 100).toFixed(1);
            ctx.fillText(`${item.name}: ${item.count} (${percent}%)`, legendX + 18, legendY);
            legendY += 20;
        });
    }

    function renderBarChart(canvasId, data) {
        const canvas = document.getElementById(canvasId);
        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 400;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        if (data.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('No data', canvas.width / 2, canvas.height / 2);
            return;
        }

        const maxCount = Math.max(...data.map(d => d.count));
        const barHeight = 28;
        const gap = 8;
        const leftMargin = 120;
        const rightMargin = 60;
        const barMaxWidth = canvas.width - leftMargin - rightMargin;

        data.forEach((item, i) => {
            const y = 30 + i * (barHeight + gap);
            const barWidth = (item.count / maxCount) * barMaxWidth;

            // Label
            ctx.fillStyle = '#42526E';
            ctx.font = '12px sans-serif';
            ctx.textAlign = 'right';
            const label = item.name.length > 15 ? item.name.substring(0, 15) + '...' : item.name;
            ctx.fillText(label, leftMargin - 10, y + barHeight / 2 + 4);

            // Bar
            ctx.fillStyle = '#0052CC';
            ctx.fillRect(leftMargin, y, barWidth, barHeight);

            // Count
            ctx.fillStyle = '#42526E';
            ctx.textAlign = 'left';
            ctx.fillText(item.count, leftMargin + barWidth + 8, y + barHeight / 2 + 4);
        });
    }

    function getChartColors(count) {
        const baseColors = [
            '#0052CC', '#36B37E', '#FFAB00', '#FF5630', '#6554C0',
            '#00B8D9', '#97A0AF', '#FF8B00', '#172B4D', '#8777D9'
        ];
        const colors = [];
        for (let i = 0; i < count; i++) {
            colors.push(baseColors[i % baseColors.length]);
        }
        return colors;
    }

    function handleChartClick(e, chartType) {
        const fieldMap = {
            'status': 'status',
            'priority': 'priority',
            'type': 'issue_type',
            'assignee': 'assignee'
        };

        // Toggle filter - for simplicity, we'll show a prompt
        const distribution = getDistribution(fieldMap[chartType]);
        if (distribution.length === 0) return;

        const values = distribution.map(d => d.name);
        const selected = prompt(`Filter by ${chartType}:\n\n${values.join('\n')}\n\nEnter value (or empty to clear):`);

        if (selected === null) return;

        if (selected === '') {
            activeChartFilter = null;
        } else if (values.includes(selected)) {
            activeChartFilter = { field: fieldMap[chartType], value: selected };
        }

        applyFilters();
    }

    function openModal(issue) {
        const modal = document.getElementById('issue-modal');

        document.getElementById('modal-issue-key').textContent = issue.key;
        document.getElementById('modal-issue-summary').textContent = issue.summary;

        const statusEl = document.getElementById('modal-status');
        statusEl.textContent = issue.status;
        statusEl.className = 'status-badge ' + getStatusClass(issue.status);

        const priorityEl = document.getElementById('modal-priority');
        priorityEl.textContent = issue.priority;
        priorityEl.className = 'priority-badge ' + getPriorityClass(issue.priority);

        document.getElementById('modal-assignee').textContent = issue.assignee;
        document.getElementById('modal-reporter').textContent = issue.reporter;
        document.getElementById('modal-type').textContent = issue.issue_type;
        document.getElementById('modal-components').textContent = issue.components.join(', ') || '-';
        document.getElementById('modal-labels').textContent = issue.labels.join(', ') || '-';
        document.getElementById('modal-created').textContent = formatDateTime(issue.created_date);
        document.getElementById('modal-updated').textContent = formatDateTime(issue.updated_date);

        // Render change history
        const timeline = document.getElementById('history-timeline');
        if (issue.change_history && issue.change_history.length > 0) {
            timeline.innerHTML = issue.change_history.map(h => `
                <div class="timeline-item">
                    <div class="timeline-date">${formatDateTime(h.changed_at)}</div>
                    <div class="timeline-field">${h.field}</div>
                    <div class="timeline-change">${h.from_string} → ${h.to_string}</div>
                    <div class="timeline-author">by ${h.author}</div>
                </div>
            `).join('');
        } else {
            timeline.innerHTML = '<p style="color: #97A0AF;">No change history available</p>';
        }

        modal.classList.add('active');
        document.body.style.overflow = 'hidden';
    }

    function closeModal() {
        document.getElementById('issue-modal').classList.remove('active');
        document.body.style.overflow = '';
    }

    function clearFilters() {
        document.getElementById('project-filter').value = '';
        document.getElementById('status-filter').value = '';
        document.getElementById('priority-filter').value = '';
        document.getElementById('assignee-filter').value = '';
        document.getElementById('type-filter').value = '';
        document.getElementById('date-from').value = '';
        document.getElementById('date-to').value = '';
        document.getElementById('search-input').value = '';
        activeChartFilter = null;
        applyFilters();
        renderCharts();
    }

    function exportCsv() {
        const headers = ['Key', 'Summary', 'Status', 'Priority', 'Assignee', 'Reporter', 'Type', 'Components', 'Labels', 'Created', 'Updated', 'Project'];
        const rows = filteredIssues.map(issue => [
            issue.key,
            `"${(issue.summary || '').replace(/"/g, '""')}"`,
            issue.status,
            issue.priority,
            issue.assignee,
            issue.reporter,
            issue.issue_type,
            `"${(issue.components || []).join(', ')}"`,
            `"${(issue.labels || []).join(', ')}"`,
            formatDate(issue.created_date),
            formatDate(issue.updated_date),
            issue.project_key
        ]);

        const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
        const blob = new Blob(['\ufeff' + csv], { type: 'text/csv;charset=utf-8;' });
        const link = document.createElement('a');
        link.href = URL.createObjectURL(blob);
        link.download = `jira-report-${new Date().toISOString().split('T')[0]}.csv`;
        link.click();
    }

    function getStatusClass(status) {
        const s = (status || '').toLowerCase();
        if (s.includes('done') || s.includes('complete') || s.includes('closed') || s.includes('resolved')) return 'status-done';
        if (s.includes('progress') || s.includes('active')) return 'status-inprogress';
        if (s.includes('review') || s.includes('testing')) return 'status-review';
        return 'status-todo';
    }

    function getPriorityClass(priority) {
        const p = (priority || '').toLowerCase();
        if (p.includes('highest') || p.includes('blocker') || p.includes('critical')) return 'priority-highest';
        if (p.includes('high') || p.includes('major')) return 'priority-high';
        if (p.includes('medium') || p.includes('normal')) return 'priority-medium';
        if (p.includes('low') || p.includes('minor')) return 'priority-low';
        if (p.includes('lowest') || p.includes('trivial')) return 'priority-lowest';
        return 'priority-medium';
    }

    function formatDate(dateStr) {
        if (!dateStr) return '-';
        return new Date(dateStr).toLocaleDateString('ja-JP');
    }

    function formatDateTime(dateStr) {
        if (!dateStr) return '-';
        return new Date(dateStr).toLocaleString('ja-JP');
    }

    function escapeHtml(str) {
        if (!str) return '';
        return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    function debounce(fn, delay) {
        let timer;
        return function(...args) {
            clearTimeout(timer);
            timer = setTimeout(() => fn.apply(this, args), delay);
        };
    }
})();
    "#
}
