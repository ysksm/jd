use crate::application::use_cases::ReportData;

pub fn generate_static_report(data: &ReportData) -> String {
    let mut html = String::new();

    // HTML header
    html.push_str(&format!(r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>JIRA Report - {}</title>
    <style>
{}
    </style>
</head>
<body>
    <div class="container">
        <header class="header">
            <h1>JIRA Report</h1>
            <p class="generated-at">Generated: {}</p>
            <p class="summary">Total Issues: {}</p>
        </header>
"#,
        data.generated_at.format("%Y-%m-%d"),
        get_static_css(),
        data.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
        data.total_issues
    ));

    // Project sections
    for project in &data.projects {
        html.push_str(&format!(r#"
        <section class="project-section">
            <h2 class="project-title">{} ({})</h2>
            <p class="issue-count">{} issues</p>

            <div class="charts-container">
                <div class="chart-box">
                    <h3>Status Distribution</h3>
                    <div class="bar-chart">
"#, project.name, project.key, project.issues.len()));

        // Status distribution
        let total = project.issues.len() as f64;
        let mut sorted_statuses: Vec<_> = project.status_counts.iter().collect();
        sorted_statuses.sort_by(|a, b| b.1.cmp(a.1));

        for (status, count) in sorted_statuses {
            let percentage = (*count as f64 / total * 100.0).round();
            let color = get_status_color(status);
            html.push_str(&format!(r#"                        <div class="bar-item">
                            <span class="bar-label">{}</span>
                            <div class="bar-container">
                                <div class="bar" style="width: {}%; background-color: {};"></div>
                            </div>
                            <span class="bar-value">{} ({}%)</span>
                        </div>
"#, status, percentage, color, count, percentage as i32));
        }

        html.push_str(r#"                    </div>
                </div>

                <div class="chart-box">
                    <h3>Priority Distribution</h3>
                    <div class="bar-chart">
"#);

        // Priority distribution
        let mut sorted_priorities: Vec<_> = project.priority_counts.iter().collect();
        sorted_priorities.sort_by(|a, b| b.1.cmp(a.1));

        for (priority, count) in sorted_priorities {
            let percentage = (*count as f64 / total * 100.0).round();
            let color = get_priority_color(priority);
            html.push_str(&format!(r#"                        <div class="bar-item">
                            <span class="bar-label">{}</span>
                            <div class="bar-container">
                                <div class="bar" style="width: {}%; background-color: {};"></div>
                            </div>
                            <span class="bar-value">{} ({}%)</span>
                        </div>
"#, priority, percentage, color, count, percentage as i32));
        }

        html.push_str(r#"                    </div>
                </div>

                <div class="chart-box">
                    <h3>Assignee Distribution</h3>
                    <div class="bar-chart">
"#);

        // Assignee distribution
        let mut sorted_assignees: Vec<_> = project.assignee_counts.iter().collect();
        sorted_assignees.sort_by(|a, b| b.1.cmp(a.1));

        for (assignee, count) in sorted_assignees.iter().take(10) {
            let percentage = (**count as f64 / total * 100.0).round();
            html.push_str(&format!(r#"                        <div class="bar-item">
                            <span class="bar-label">{}</span>
                            <div class="bar-container">
                                <div class="bar" style="width: {}%; background-color: #0052CC;"></div>
                            </div>
                            <span class="bar-value">{} ({}%)</span>
                        </div>
"#, assignee, percentage, count, percentage as i32));
        }

        html.push_str(r#"                    </div>
                </div>

                <div class="chart-box">
                    <h3>Issue Type Distribution</h3>
                    <div class="bar-chart">
"#);

        // Issue type distribution
        let mut sorted_types: Vec<_> = project.issue_type_counts.iter().collect();
        sorted_types.sort_by(|a, b| b.1.cmp(a.1));

        for (issue_type, count) in sorted_types {
            let percentage = (*count as f64 / total * 100.0).round();
            let color = get_issue_type_color(issue_type);
            html.push_str(&format!(r#"                        <div class="bar-item">
                            <span class="bar-label">{}</span>
                            <div class="bar-container">
                                <div class="bar" style="width: {}%; background-color: {};"></div>
                            </div>
                            <span class="bar-value">{} ({}%)</span>
                        </div>
"#, issue_type, percentage, color, count, percentage as i32));
        }

        html.push_str(r#"                    </div>
                </div>
            </div>

            <h3 class="table-title">Issue List</h3>
            <table class="issue-table">
                <thead>
                    <tr>
                        <th>Key</th>
                        <th>Summary</th>
                        <th>Status</th>
                        <th>Priority</th>
                        <th>Assignee</th>
                        <th>Type</th>
                        <th>Created</th>
                    </tr>
                </thead>
                <tbody>
"#);

        // Issue list
        for issue in &project.issues {
            let created = issue.created_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string());
            let status_class = get_status_class(&issue.status);
            let priority_class = get_priority_class(&issue.priority);

            html.push_str(&format!(r#"                    <tr>
                        <td class="issue-key">{}</td>
                        <td class="issue-summary">{}</td>
                        <td><span class="status-badge {}">{}</span></td>
                        <td><span class="priority-badge {}">{}</span></td>
                        <td>{}</td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>
"#,
                issue.key,
                html_escape(&issue.summary),
                status_class,
                issue.status,
                priority_class,
                issue.priority,
                issue.assignee,
                issue.issue_type,
                created
            ));
        }

        html.push_str(r#"                </tbody>
            </table>
        </section>
"#);
    }

    // Footer
    html.push_str(r#"
        <footer class="footer">
            <p>Generated by jira-db</p>
        </footer>
    </div>
</body>
</html>
"#);

    html
}

fn get_static_css() -> &'static str {
    r#"
        :root {
            --jira-blue: #0052CC;
            --jira-blue-light: #DEEBFF;
            --jira-green: #36B37E;
            --jira-yellow: #FFAB00;
            --jira-red: #FF5630;
            --jira-purple: #6554C0;
            --jira-teal: #00B8D9;
            --jira-gray: #42526E;
            --jira-gray-light: #97A0AF;
            --jira-bg: #FAFBFC;
            --jira-border: #DFE1E6;
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

        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 24px;
        }

        .header {
            background: linear-gradient(135deg, var(--jira-blue), #0747A6);
            color: white;
            padding: 32px;
            border-radius: 8px;
            margin-bottom: 24px;
        }

        .header h1 {
            font-size: 28px;
            margin-bottom: 8px;
        }

        .generated-at, .summary {
            opacity: 0.9;
            font-size: 14px;
        }

        .project-section {
            background: white;
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            padding: 24px;
            margin-bottom: 24px;
        }

        .project-title {
            color: var(--jira-blue);
            font-size: 22px;
            margin-bottom: 4px;
        }

        .issue-count {
            color: var(--jira-gray-light);
            margin-bottom: 24px;
        }

        .charts-container {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 24px;
            margin-bottom: 32px;
        }

        .chart-box {
            background: var(--jira-bg);
            border: 1px solid var(--jira-border);
            border-radius: 8px;
            padding: 16px;
        }

        .chart-box h3 {
            font-size: 14px;
            color: var(--jira-gray);
            margin-bottom: 16px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .bar-chart {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .bar-item {
            display: grid;
            grid-template-columns: 100px 1fr 80px;
            gap: 8px;
            align-items: center;
        }

        .bar-label {
            font-size: 13px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        .bar-container {
            height: 20px;
            background: var(--jira-border);
            border-radius: 4px;
            overflow: hidden;
        }

        .bar {
            height: 100%;
            border-radius: 4px;
            transition: width 0.3s ease;
        }

        .bar-value {
            font-size: 12px;
            color: var(--jira-gray-light);
            text-align: right;
        }

        .table-title {
            font-size: 16px;
            margin-bottom: 16px;
            color: var(--jira-gray);
        }

        .issue-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 14px;
        }

        .issue-table th {
            background: var(--jira-bg);
            text-align: left;
            padding: 12px 8px;
            border-bottom: 2px solid var(--jira-border);
            font-weight: 600;
            color: var(--jira-gray);
        }

        .issue-table td {
            padding: 12px 8px;
            border-bottom: 1px solid var(--jira-border);
        }

        .issue-table tr:hover {
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
            padding: 2px 8px;
            border-radius: 3px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
        }

        .status-done { background: #E3FCEF; color: #006644; }
        .status-inprogress { background: #DEEBFF; color: #0747A6; }
        .status-todo { background: #DFE1E6; color: #42526E; }
        .status-review { background: #EAE6FF; color: #403294; }

        .priority-highest { background: #FFEBE6; color: #BF2600; }
        .priority-high { background: #FFEBE6; color: #DE350B; }
        .priority-medium { background: #FFFAE6; color: #FF8B00; }
        .priority-low { background: #E3FCEF; color: #006644; }
        .priority-lowest { background: #F4F5F7; color: #42526E; }

        .footer {
            text-align: center;
            padding: 24px;
            color: var(--jira-gray-light);
            font-size: 12px;
        }

        @media print {
            .container {
                max-width: none;
                padding: 0;
            }

            .project-section {
                break-inside: avoid;
                box-shadow: none;
                border: 1px solid var(--jira-border);
            }

            .issue-table tr:hover {
                background: none;
            }
        }

        @media (max-width: 768px) {
            .charts-container {
                grid-template-columns: 1fr;
            }

            .bar-item {
                grid-template-columns: 80px 1fr 60px;
            }

            .issue-table {
                font-size: 12px;
            }

            .issue-summary {
                max-width: 200px;
            }
        }
    "#
}

fn get_status_color(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("complete") || s.contains("closed") || s.contains("resolved") => "#36B37E",
        s if s.contains("progress") || s.contains("active") => "#0052CC",
        s if s.contains("review") || s.contains("testing") => "#6554C0",
        s if s.contains("blocked") || s.contains("impediment") => "#FF5630",
        _ => "#97A0AF",
    }
}

fn get_priority_color(priority: &str) -> &'static str {
    match priority.to_lowercase().as_str() {
        s if s.contains("highest") || s.contains("blocker") || s.contains("critical") => "#FF5630",
        s if s.contains("high") || s.contains("major") => "#FF7452",
        s if s.contains("medium") || s.contains("normal") => "#FFAB00",
        s if s.contains("low") || s.contains("minor") => "#36B37E",
        s if s.contains("lowest") || s.contains("trivial") => "#97A0AF",
        _ => "#97A0AF",
    }
}

fn get_issue_type_color(issue_type: &str) -> &'static str {
    match issue_type.to_lowercase().as_str() {
        s if s.contains("bug") => "#FF5630",
        s if s.contains("story") => "#36B37E",
        s if s.contains("task") => "#0052CC",
        s if s.contains("epic") => "#6554C0",
        s if s.contains("subtask") || s.contains("sub-task") => "#00B8D9",
        _ => "#97A0AF",
    }
}

fn get_status_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        s if s.contains("done") || s.contains("complete") || s.contains("closed") || s.contains("resolved") => "status-done",
        s if s.contains("progress") || s.contains("active") => "status-inprogress",
        s if s.contains("review") || s.contains("testing") => "status-review",
        _ => "status-todo",
    }
}

fn get_priority_class(priority: &str) -> &'static str {
    match priority.to_lowercase().as_str() {
        s if s.contains("highest") || s.contains("blocker") || s.contains("critical") => "priority-highest",
        s if s.contains("high") || s.contains("major") => "priority-high",
        s if s.contains("medium") || s.contains("normal") => "priority-medium",
        s if s.contains("low") || s.contains("minor") => "priority-low",
        s if s.contains("lowest") || s.contains("trivial") => "priority-lowest",
        _ => "priority-medium",
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
