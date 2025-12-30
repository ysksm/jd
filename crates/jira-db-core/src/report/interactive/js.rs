/// インタラクティブレポート用JavaScript
pub fn get_js() -> &'static str {
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
    let currentTab = 'daily-scrum';

    // Initialize
    document.addEventListener('DOMContentLoaded', init);

    function init() {
        parseData();
        populateFilters();
        bindEvents();
        bindTabEvents();
        applyFilters();
        renderCurrentTab();
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
        const sprints = new Set();

        allIssues.forEach(issue => {
            projects.add(issue.project_key);
            statuses.add(issue.status);
            priorities.add(issue.priority);
            assignees.add(issue.assignee);
            types.add(issue.issue_type);
            if (issue.sprint) sprints.add(issue.sprint);
        });

        populateSelect('project-filter', Array.from(projects).sort());
        populateSelect('status-filter', Array.from(statuses).sort());
        populateSelect('priority-filter', Array.from(priorities).sort());
        populateSelect('assignee-filter', Array.from(assignees).sort());
        populateSelect('type-filter', Array.from(types).sort());
        populateSelect('sprint-filter', Array.from(sprints).sort());
    }

    function populateSelect(id, options) {
        const select = document.getElementById(id);
        if (!select) return;
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

    function bindTabEvents() {
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const tabId = btn.dataset.tab;
                switchTab(tabId);
            });
        });
    }

    function switchTab(tabId) {
        currentTab = tabId;

        // Update tab buttons
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.tab === tabId);
        });

        // Update tab contents
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.toggle('active', content.id === 'tab-' + tabId);
        });

        renderCurrentTab();
    }

    function renderCurrentTab() {
        switch(currentTab) {
            case 'daily-scrum':
                renderDailyScrumTab();
                break;
            case 'sprint-planning':
                renderSprintPlanningTab();
                break;
            case 'sprint-board':
                renderSprintBoardTab();
                break;
            case 'bug-tracker':
                renderBugTrackerTab();
                break;
            case 'retrospective':
                renderRetrospectiveTab();
                break;
            case 'trends':
                renderTrendsTab();
                break;
        }
    }

    // ===== デイリースクラムタブ =====
    function renderDailyScrumTab() {
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        const yesterday = new Date(today);
        yesterday.setDate(yesterday.getDate() - 1);

        // 昨日完了したチケット
        const completedYesterday = filteredIssues.filter(issue => {
            if (!issue.updated_date) return false;
            const updated = new Date(issue.updated_date);
            updated.setHours(0, 0, 0, 0);
            return updated.getTime() === yesterday.getTime() && isDone(issue.status);
        });

        // 今日作業中のチケット
        const inProgressToday = filteredIssues.filter(issue => {
            return isInProgress(issue.status);
        });

        // ブロッカー（優先度がHighest/Blockerのチケット）
        const blockers = filteredIssues.filter(issue => {
            const p = (issue.priority || '').toLowerCase();
            return (p.includes('highest') || p.includes('blocker') || p.includes('critical')) && !isDone(issue.status);
        });

        // 統計を更新
        updateElement('daily-completed-count', completedYesterday.length);
        updateElement('daily-inprogress-count', inProgressToday.length);
        updateElement('daily-blockers-count', blockers.length);

        // リストを更新
        renderIssueList('yesterday-list', completedYesterday.slice(0, 10));
        renderIssueList('today-list', inProgressToday.slice(0, 10));
        renderIssueList('blockers-list', blockers.slice(0, 10));
    }

    // ===== スプリント計画タブ =====
    function renderSprintPlanningTab() {
        // バックログ = スプリントが未設定のチケット
        const backlog = filteredIssues.filter(issue => {
            return !issue.sprint && !isDone(issue.status);
        });

        // 優先度別にグループ化
        const byPriority = {};
        backlog.forEach(issue => {
            const priority = issue.priority || 'Unknown';
            if (!byPriority[priority]) byPriority[priority] = [];
            byPriority[priority].push(issue);
        });

        // 統計を更新
        updateElement('backlog-total', backlog.length);
        updateElement('backlog-bugs', backlog.filter(i => isBug(i.issue_type)).length);
        updateElement('backlog-stories', backlog.filter(i => isStory(i.issue_type)).length);

        // バックログテーブルを更新
        const tbody = document.getElementById('backlog-tbody');
        if (tbody) {
            tbody.innerHTML = backlog.slice(0, 50).map(issue => `
                <tr data-key="${issue.key}">
                    <td class="issue-key">${issue.key}</td>
                    <td class="issue-summary" title="${escapeHtml(issue.summary)}">${escapeHtml(issue.summary)}</td>
                    <td><span class="priority-badge ${getPriorityClass(issue.priority)}">${issue.priority}</span></td>
                    <td>${issue.issue_type}</td>
                    <td>${issue.assignee}</td>
                    <td>${formatDate(issue.created_date)}</td>
                </tr>
            `).join('');

            bindRowClickEvents(tbody);
        }
    }

    // ===== スプリントボードタブ =====
    function renderSprintBoardTab() {
        // 現在のスプリントを取得（最新のスプリント）
        const sprints = [...new Set(filteredIssues.filter(i => i.sprint).map(i => i.sprint))].sort().reverse();
        const currentSprint = sprints[0] || null;

        updateElement('current-sprint-name', currentSprint || 'スプリントなし');

        if (!currentSprint) {
            updateElement('sprint-total', 0);
            updateElement('sprint-done', 0);
            updateElement('sprint-inprogress', 0);
            updateElement('sprint-todo', 0);
            updateElement('sprint-carryover', 0);
            return;
        }

        const sprintIssues = filteredIssues.filter(i => i.sprint === currentSprint);
        const done = sprintIssues.filter(i => isDone(i.status));
        const inProgress = sprintIssues.filter(i => isInProgress(i.status));
        const todo = sprintIssues.filter(i => !isDone(i.status) && !isInProgress(i.status));

        // キャリーオーバー検知（前スプリントから持ち越されたチケット）
        const previousSprint = sprints[1] || null;
        let carryover = [];
        if (previousSprint) {
            // 変更履歴からスプリント変更を検知
            carryover = sprintIssues.filter(issue => {
                if (!issue.change_history) return false;
                return issue.change_history.some(h =>
                    h.field.toLowerCase() === 'sprint' &&
                    h.from_string && h.from_string.includes(previousSprint)
                );
            });
        }

        updateElement('sprint-total', sprintIssues.length);
        updateElement('sprint-done', done.length);
        updateElement('sprint-inprogress', inProgress.length);
        updateElement('sprint-todo', todo.length);
        updateElement('sprint-carryover', carryover.length);

        // バーンダウンチャートを描画
        renderSprintBurndown(sprintIssues);

        // スプリントチケットテーブル
        const tbody = document.getElementById('sprint-tbody');
        if (tbody) {
            tbody.innerHTML = sprintIssues.map(issue => {
                const isCarryover = carryover.some(c => c.key === issue.key);
                return `
                <tr data-key="${issue.key}">
                    <td class="issue-key">${issue.key}${isCarryover ? '<span class="carryover-badge">持越</span>' : ''}</td>
                    <td class="issue-summary" title="${escapeHtml(issue.summary)}">${escapeHtml(issue.summary)}</td>
                    <td><span class="status-badge ${getStatusClass(issue.status)}">${issue.status}</span></td>
                    <td><span class="priority-badge ${getPriorityClass(issue.priority)}">${issue.priority}</span></td>
                    <td>${issue.assignee}</td>
                </tr>
            `}).join('');

            bindRowClickEvents(tbody);
        }
    }

    function renderSprintBurndown(sprintIssues) {
        const canvas = document.getElementById('sprint-burndown-chart');
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 400;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        if (sprintIssues.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('スプリントデータがありません', canvas.width / 2, canvas.height / 2);
            return;
        }

        // スプリント期間中の日別完了数を計算
        const dates = new Map();
        const totalIssues = sprintIssues.length;

        sprintIssues.forEach(issue => {
            if (isDone(issue.status) && issue.updated_date) {
                const date = issue.updated_date.split('T')[0];
                dates.set(date, (dates.get(date) || 0) + 1);
            }
        });

        const sortedDates = [...dates.keys()].sort();
        if (sortedDates.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('完了したチケットがありません', canvas.width / 2, canvas.height / 2);
            return;
        }

        // 累積完了数を計算
        let cumulative = 0;
        const burndownData = sortedDates.map(date => {
            cumulative += dates.get(date);
            return { date, remaining: totalIssues - cumulative };
        });

        // 描画
        const padding = { top: 40, right: 40, bottom: 60, left: 70 };
        const chartWidth = canvas.width - padding.left - padding.right;
        const chartHeight = canvas.height - padding.top - padding.bottom;

        const scaleX = (i) => padding.left + (i / (burndownData.length - 1 || 1)) * chartWidth;
        const scaleY = (val) => padding.top + chartHeight - (val / totalIssues) * chartHeight;

        // グリッド線
        ctx.strokeStyle = '#DFE1E6';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 5; i++) {
            const y = padding.top + (i / 5) * chartHeight;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(padding.left + chartWidth, y);
            ctx.stroke();

            const value = Math.round(totalIssues * (1 - i / 5));
            ctx.fillStyle = '#97A0AF';
            ctx.font = '11px sans-serif';
            ctx.textAlign = 'right';
            ctx.fillText(value.toString(), padding.left - 10, y + 4);
        }

        // 理想線
        ctx.beginPath();
        ctx.strokeStyle = '#DFE1E6';
        ctx.setLineDash([5, 5]);
        ctx.lineWidth = 2;
        ctx.moveTo(scaleX(0), scaleY(totalIssues));
        ctx.lineTo(scaleX(burndownData.length - 1), scaleY(0));
        ctx.stroke();
        ctx.setLineDash([]);

        // 実績線
        ctx.beginPath();
        ctx.strokeStyle = '#FF5630';
        ctx.lineWidth = 3;
        ctx.moveTo(scaleX(0), scaleY(totalIssues));
        burndownData.forEach((point, i) => {
            ctx.lineTo(scaleX(i), scaleY(point.remaining));
        });
        ctx.stroke();
    }

    // ===== バグトラッカータブ =====
    function renderBugTrackerTab() {
        const bugs = filteredIssues.filter(issue => isBug(issue.issue_type));
        const openBugs = bugs.filter(b => !isDone(b.status));
        const closedBugs = bugs.filter(b => isDone(b.status));

        // 経過日数を計算
        const now = new Date();
        const bugsWithAge = openBugs.map(bug => {
            const created = new Date(bug.created_date);
            const age = Math.floor((now - created) / (1000 * 60 * 60 * 24));
            return { ...bug, age };
        });

        // 優先度別
        const critical = bugsWithAge.filter(b => {
            const p = (b.priority || '').toLowerCase();
            return p.includes('highest') || p.includes('blocker') || p.includes('critical');
        });
        const high = bugsWithAge.filter(b => {
            const p = (b.priority || '').toLowerCase();
            return p.includes('high') || p.includes('major');
        });

        // 統計
        updateElement('bug-total', bugs.length);
        updateElement('bug-open', openBugs.length);
        updateElement('bug-critical', critical.length);
        updateElement('bug-old', bugsWithAge.filter(b => b.age > 30).length);

        // バグテーブル
        const tbody = document.getElementById('bug-tbody');
        if (tbody) {
            bugsWithAge.sort((a, b) => {
                // 優先度でソート、その後経過日数
                const priorityOrder = getPriorityOrder(b.priority) - getPriorityOrder(a.priority);
                if (priorityOrder !== 0) return priorityOrder;
                return b.age - a.age;
            });

            tbody.innerHTML = bugsWithAge.slice(0, 50).map(bug => `
                <tr data-key="${bug.key}">
                    <td class="issue-key">${bug.key}</td>
                    <td class="issue-summary" title="${escapeHtml(bug.summary)}">${escapeHtml(bug.summary)}</td>
                    <td><span class="status-badge ${getStatusClass(bug.status)}">${bug.status}</span></td>
                    <td><span class="priority-badge ${getPriorityClass(bug.priority)}">${bug.priority}</span></td>
                    <td>${bug.assignee}</td>
                    <td>${bug.age}日<span class="age-badge ${getAgeClass(bug.age)}">${getAgeLabel(bug.age)}</span></td>
                </tr>
            `).join('');

            bindRowClickEvents(tbody);
        }

        // バグ優先度チャート
        renderBugPriorityChart(bugsWithAge);
    }

    function renderBugPriorityChart(bugs) {
        const canvas = document.getElementById('bug-priority-chart');
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 300;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        const priorityCounts = {};
        bugs.forEach(bug => {
            const p = bug.priority || 'Unknown';
            priorityCounts[p] = (priorityCounts[p] || 0) + 1;
        });

        const data = Object.entries(priorityCounts).map(([name, count]) => ({ name, count }));
        data.sort((a, b) => getPriorityOrder(b.name) - getPriorityOrder(a.name));

        if (data.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('バグがありません', canvas.width / 2, canvas.height / 2);
            return;
        }

        const maxCount = Math.max(...data.map(d => d.count));
        const barHeight = 28;
        const gap = 8;
        const leftMargin = 100;
        const rightMargin = 60;
        const barMaxWidth = canvas.width - leftMargin - rightMargin;

        data.forEach((item, i) => {
            const y = 30 + i * (barHeight + gap);
            const barWidth = (item.count / maxCount) * barMaxWidth;

            ctx.fillStyle = '#42526E';
            ctx.font = '12px sans-serif';
            ctx.textAlign = 'right';
            ctx.fillText(item.name, leftMargin - 10, y + barHeight / 2 + 4);

            ctx.fillStyle = getPriorityColor(item.name);
            ctx.fillRect(leftMargin, y, barWidth, barHeight);

            ctx.fillStyle = '#42526E';
            ctx.textAlign = 'left';
            ctx.fillText(item.count, leftMargin + barWidth + 8, y + barHeight / 2 + 4);
        });
    }

    // ===== 振り返りタブ =====
    function renderRetrospectiveTab() {
        // 完了したスプリントを取得
        const sprints = [...new Set(filteredIssues.filter(i => i.sprint).map(i => i.sprint))].sort().reverse();
        const lastSprint = sprints[0] || null;

        updateElement('retro-sprint-name', lastSprint || 'スプリントなし');

        if (!lastSprint) {
            updateElement('retro-completed', 0);
            updateElement('retro-velocity', 0);
            updateElement('retro-bugs-found', 0);
            updateElement('retro-carryover-next', 0);
            return;
        }

        const sprintIssues = filteredIssues.filter(i => i.sprint === lastSprint);
        const completed = sprintIssues.filter(i => isDone(i.status));
        const notCompleted = sprintIssues.filter(i => !isDone(i.status));
        const bugsFound = sprintIssues.filter(i => isBug(i.issue_type));

        updateElement('retro-completed', completed.length);
        updateElement('retro-velocity', completed.length); // ストーリーポイントなしのためチケット数
        updateElement('retro-bugs-found', bugsFound.length);
        updateElement('retro-carryover-next', notCompleted.length);

        // 完了チケット分布チャート
        renderRetroCompletionChart(sprintIssues);

        // 担当者別パフォーマンス
        const tbody = document.getElementById('retro-assignee-tbody');
        if (tbody) {
            const assigneeStats = {};
            sprintIssues.forEach(issue => {
                const assignee = issue.assignee || 'Unassigned';
                if (!assigneeStats[assignee]) {
                    assigneeStats[assignee] = { total: 0, completed: 0 };
                }
                assigneeStats[assignee].total++;
                if (isDone(issue.status)) {
                    assigneeStats[assignee].completed++;
                }
            });

            const rows = Object.entries(assigneeStats)
                .map(([name, stats]) => ({
                    name,
                    total: stats.total,
                    completed: stats.completed,
                    rate: stats.total > 0 ? Math.round(stats.completed / stats.total * 100) : 0
                }))
                .sort((a, b) => b.completed - a.completed);

            tbody.innerHTML = rows.map(row => `
                <tr>
                    <td>${row.name}</td>
                    <td>${row.total}</td>
                    <td>${row.completed}</td>
                    <td>${row.rate}%</td>
                </tr>
            `).join('');
        }
    }

    function renderRetroCompletionChart(sprintIssues) {
        const canvas = document.getElementById('retro-completion-chart');
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 300;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        const completed = sprintIssues.filter(i => isDone(i.status)).length;
        const inProgress = sprintIssues.filter(i => isInProgress(i.status)).length;
        const todo = sprintIssues.filter(i => !isDone(i.status) && !isInProgress(i.status)).length;

        const data = [
            { name: '完了', count: completed, color: '#36B37E' },
            { name: '進行中', count: inProgress, color: '#0052CC' },
            { name: '未着手', count: todo, color: '#DFE1E6' }
        ].filter(d => d.count > 0);

        if (data.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('データがありません', canvas.width / 2, canvas.height / 2);
            return;
        }

        const total = data.reduce((sum, d) => sum + d.count, 0);
        const centerX = canvas.width / 4;
        const centerY = canvas.height / 2;
        const radius = Math.min(centerX, centerY) - 20;

        let startAngle = -Math.PI / 2;
        data.forEach(item => {
            const sliceAngle = (item.count / total) * 2 * Math.PI;
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.arc(centerX, centerY, radius, startAngle, startAngle + sliceAngle);
            ctx.closePath();
            ctx.fillStyle = item.color;
            ctx.fill();
            startAngle += sliceAngle;
        });

        // 凡例
        let legendY = 50;
        data.forEach(item => {
            ctx.fillStyle = item.color;
            ctx.fillRect(canvas.width / 2 + 20, legendY - 10, 12, 12);
            ctx.fillStyle = '#42526E';
            ctx.font = '12px sans-serif';
            ctx.textAlign = 'left';
            const percent = ((item.count / total) * 100).toFixed(1);
            ctx.fillText(`${item.name}: ${item.count} (${percent}%)`, canvas.width / 2 + 40, legendY);
            legendY += 25;
        });
    }

    // ===== トレンドタブ =====
    function renderTrendsTab() {
        // 月別チケット作成数・解決数
        const monthlyData = {};
        filteredIssues.forEach(issue => {
            if (issue.created_date) {
                const month = issue.created_date.substring(0, 7); // YYYY-MM
                if (!monthlyData[month]) {
                    monthlyData[month] = { created: 0, resolved: 0 };
                }
                monthlyData[month].created++;
            }
            if (issue.updated_date && isDone(issue.status)) {
                const month = issue.updated_date.substring(0, 7);
                if (!monthlyData[month]) {
                    monthlyData[month] = { created: 0, resolved: 0 };
                }
                monthlyData[month].resolved++;
            }
        });

        const months = Object.keys(monthlyData).sort();
        const trendData = months.map(month => ({
            month,
            created: monthlyData[month].created,
            resolved: monthlyData[month].resolved
        }));

        // 統計
        const totalCreated = filteredIssues.length;
        const totalResolved = filteredIssues.filter(i => isDone(i.status)).length;
        const avgPerMonth = months.length > 0 ? Math.round(totalCreated / months.length) : 0;
        const resolutionRate = totalCreated > 0 ? Math.round(totalResolved / totalCreated * 100) : 0;

        updateElement('trend-total-created', totalCreated);
        updateElement('trend-total-resolved', totalResolved);
        updateElement('trend-avg-per-month', avgPerMonth);
        updateElement('trend-resolution-rate', resolutionRate + '%');

        // トレンドチャート描画
        renderTrendsChart(trendData);

        // タイプ別トレンド
        renderTypeTrendsChart();
    }

    function renderTrendsChart(data) {
        const canvas = document.getElementById('trends-chart');
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 400;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        if (data.length === 0) {
            ctx.fillStyle = '#97A0AF';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('データがありません', canvas.width / 2, canvas.height / 2);
            return;
        }

        const padding = { top: 40, right: 40, bottom: 80, left: 70 };
        const chartWidth = canvas.width - padding.left - padding.right;
        const chartHeight = canvas.height - padding.top - padding.bottom;

        const maxValue = Math.max(...data.map(d => Math.max(d.created, d.resolved)), 1);

        const barWidth = chartWidth / data.length / 2.5;
        const groupWidth = chartWidth / data.length;

        // グリッド線
        ctx.strokeStyle = '#DFE1E6';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 5; i++) {
            const y = padding.top + (i / 5) * chartHeight;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(padding.left + chartWidth, y);
            ctx.stroke();

            const value = Math.round(maxValue * (1 - i / 5));
            ctx.fillStyle = '#97A0AF';
            ctx.font = '11px sans-serif';
            ctx.textAlign = 'right';
            ctx.fillText(value.toString(), padding.left - 10, y + 4);
        }

        // バー描画
        data.forEach((point, i) => {
            const x = padding.left + i * groupWidth + groupWidth / 4;

            // 作成
            const createdHeight = (point.created / maxValue) * chartHeight;
            ctx.fillStyle = '#0052CC';
            ctx.fillRect(x, padding.top + chartHeight - createdHeight, barWidth, createdHeight);

            // 解決
            const resolvedHeight = (point.resolved / maxValue) * chartHeight;
            ctx.fillStyle = '#36B37E';
            ctx.fillRect(x + barWidth + 4, padding.top + chartHeight - resolvedHeight, barWidth, resolvedHeight);

            // ラベル
            ctx.fillStyle = '#97A0AF';
            ctx.font = '10px sans-serif';
            ctx.textAlign = 'center';
            ctx.save();
            ctx.translate(x + barWidth, padding.top + chartHeight + 20);
            ctx.rotate(-Math.PI / 4);
            ctx.fillText(point.month, 0, 0);
            ctx.restore();
        });

        // 凡例
        ctx.fillStyle = '#0052CC';
        ctx.fillRect(padding.left, padding.top - 25, 12, 12);
        ctx.fillStyle = '#42526E';
        ctx.font = '12px sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText('作成', padding.left + 16, padding.top - 15);

        ctx.fillStyle = '#36B37E';
        ctx.fillRect(padding.left + 80, padding.top - 25, 12, 12);
        ctx.fillStyle = '#42526E';
        ctx.fillText('解決', padding.left + 96, padding.top - 15);
    }

    function renderTypeTrendsChart() {
        const canvas = document.getElementById('type-trends-chart');
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * 2;
        canvas.height = 300;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        const typeCounts = {};
        filteredIssues.forEach(issue => {
            const type = issue.issue_type || 'Unknown';
            typeCounts[type] = (typeCounts[type] || 0) + 1;
        });

        const data = Object.entries(typeCounts)
            .map(([name, count]) => ({ name, count }))
            .sort((a, b) => b.count - a.count);

        if (data.length === 0) return;

        const total = data.reduce((sum, d) => sum + d.count, 0);
        const colors = ['#0052CC', '#36B37E', '#FFAB00', '#FF5630', '#6554C0', '#00B8D9', '#97A0AF'];

        const centerX = canvas.width / 4;
        const centerY = canvas.height / 2;
        const radius = Math.min(centerX, centerY) - 20;

        let startAngle = -Math.PI / 2;
        data.forEach((item, i) => {
            const sliceAngle = (item.count / total) * 2 * Math.PI;
            ctx.beginPath();
            ctx.moveTo(centerX, centerY);
            ctx.arc(centerX, centerY, radius, startAngle, startAngle + sliceAngle);
            ctx.closePath();
            ctx.fillStyle = colors[i % colors.length];
            ctx.fill();
            startAngle += sliceAngle;
        });

        let legendY = 30;
        data.forEach((item, i) => {
            ctx.fillStyle = colors[i % colors.length];
            ctx.fillRect(canvas.width / 2 + 20, legendY - 10, 12, 12);
            ctx.fillStyle = '#42526E';
            ctx.font = '12px sans-serif';
            ctx.textAlign = 'left';
            const percent = ((item.count / total) * 100).toFixed(1);
            ctx.fillText(`${item.name}: ${item.count} (${percent}%)`, canvas.width / 2 + 40, legendY);
            legendY += 22;
        });
    }

    // ===== イベントバインド =====
    function bindEvents() {
        // フィルターイベント
        ['project-filter', 'status-filter', 'priority-filter', 'assignee-filter', 'type-filter', 'sprint-filter'].forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.addEventListener('change', () => {
                    activeChartFilter = null;
                    applyFilters();
                    renderCurrentTab();
                });
            }
        });

        const dateFrom = document.getElementById('date-from');
        const dateTo = document.getElementById('date-to');
        if (dateFrom) dateFrom.addEventListener('change', () => { applyFilters(); renderCurrentTab(); });
        if (dateTo) dateTo.addEventListener('change', () => { applyFilters(); renderCurrentTab(); });

        const searchInput = document.getElementById('search-input');
        if (searchInput) {
            searchInput.addEventListener('input', debounce(() => {
                applyFilters();
                renderCurrentTab();
            }, 300));
        }

        const clearBtn = document.getElementById('clear-filters');
        if (clearBtn) clearBtn.addEventListener('click', clearFilters);

        const exportBtn = document.getElementById('export-csv');
        if (exportBtn) exportBtn.addEventListener('click', exportCsv);

        // モーダルイベント
        const modalClose = document.getElementById('modal-close');
        if (modalClose) modalClose.addEventListener('click', closeModal);

        const modal = document.getElementById('issue-modal');
        if (modal) {
            modal.addEventListener('click', (e) => {
                if (e.target.id === 'issue-modal') closeModal();
            });
        }

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') closeModal();
        });
    }

    function applyFilters() {
        const projectFilter = getFilterValue('project-filter');
        const statusFilter = getFilterValue('status-filter');
        const priorityFilter = getFilterValue('priority-filter');
        const assigneeFilter = getFilterValue('assignee-filter');
        const typeFilter = getFilterValue('type-filter');
        const sprintFilter = getFilterValue('sprint-filter');
        const dateFrom = getFilterValue('date-from');
        const dateTo = getFilterValue('date-to');
        const searchQuery = (getFilterValue('search-input') || '').toLowerCase();

        filteredIssues = allIssues.filter(issue => {
            if (projectFilter && issue.project_key !== projectFilter) return false;
            if (statusFilter && issue.status !== statusFilter) return false;
            if (priorityFilter && issue.priority !== priorityFilter) return false;
            if (assigneeFilter && issue.assignee !== assigneeFilter) return false;
            if (typeFilter && issue.issue_type !== typeFilter) return false;
            if (sprintFilter && issue.sprint !== sprintFilter) return false;

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

        updateFilteredCount();
    }

    function getFilterValue(id) {
        const el = document.getElementById(id);
        return el ? el.value : '';
    }

    function updateFilteredCount() {
        updateElement('filtered-count', `(${filteredIssues.length} / ${allIssues.length})`);
    }

    function clearFilters() {
        ['project-filter', 'status-filter', 'priority-filter', 'assignee-filter', 'type-filter', 'sprint-filter', 'date-from', 'date-to', 'search-input'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.value = '';
        });
        activeChartFilter = null;
        applyFilters();
        renderCurrentTab();
    }

    function exportCsv() {
        const headers = ['Key', 'Summary', 'Status', 'Priority', 'Assignee', 'Reporter', 'Type', 'Sprint', 'Components', 'Labels', 'Created', 'Updated', 'Project'];
        const rows = filteredIssues.map(issue => [
            issue.key,
            `"${(issue.summary || '').replace(/"/g, '""')}"`,
            issue.status,
            issue.priority,
            issue.assignee,
            issue.reporter,
            issue.issue_type,
            issue.sprint || '',
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

    // ===== ヘルパー関数 =====
    function updateElement(id, value) {
        const el = document.getElementById(id);
        if (el) el.textContent = value;
    }

    function renderIssueList(id, issues) {
        const list = document.getElementById(id);
        if (!list) return;

        if (issues.length === 0) {
            list.innerHTML = '<li style="color: #97A0AF;">該当なし</li>';
            return;
        }

        list.innerHTML = issues.map(issue => `
            <li>
                <span class="issue-key" style="cursor:pointer;" onclick="window.showIssue('${issue.key}')">${issue.key}</span>
                - ${escapeHtml(issue.summary.substring(0, 50))}${issue.summary.length > 50 ? '...' : ''}
            </li>
        `).join('');
    }

    function bindRowClickEvents(tbody) {
        tbody.querySelectorAll('tr').forEach(tr => {
            tr.addEventListener('click', () => {
                const key = tr.dataset.key;
                const issue = allIssues.find(i => i.key === key);
                if (issue) openModal(issue);
            });
        });
    }

    window.showIssue = function(key) {
        const issue = allIssues.find(i => i.key === key);
        if (issue) openModal(issue);
    };

    function openModal(issue) {
        const modal = document.getElementById('issue-modal');
        if (!modal) return;

        updateElement('modal-issue-key', issue.key);
        updateElement('modal-issue-summary', issue.summary);

        const statusEl = document.getElementById('modal-status');
        if (statusEl) {
            statusEl.textContent = issue.status;
            statusEl.className = 'status-badge ' + getStatusClass(issue.status);
        }

        const priorityEl = document.getElementById('modal-priority');
        if (priorityEl) {
            priorityEl.textContent = issue.priority;
            priorityEl.className = 'priority-badge ' + getPriorityClass(issue.priority);
        }

        updateElement('modal-assignee', issue.assignee);
        updateElement('modal-reporter', issue.reporter);
        updateElement('modal-type', issue.issue_type);
        updateElement('modal-sprint', issue.sprint || '-');
        updateElement('modal-components', (issue.components || []).join(', ') || '-');
        updateElement('modal-labels', (issue.labels || []).join(', ') || '-');
        updateElement('modal-created', formatDateTime(issue.created_date));
        updateElement('modal-updated', formatDateTime(issue.updated_date));

        // 変更履歴
        const timeline = document.getElementById('history-timeline');
        if (timeline) {
            if (issue.change_history && issue.change_history.length > 0) {
                timeline.innerHTML = issue.change_history.map(h => `
                    <div class="timeline-item">
                        <div class="timeline-date">${formatDateTime(h.changed_at)}</div>
                        <div class="timeline-field">${h.field}</div>
                        <div class="timeline-change">${h.from_string || '(なし)'} → ${h.to_string || '(なし)'}</div>
                        <div class="timeline-author">by ${h.author}</div>
                    </div>
                `).join('');
            } else {
                timeline.innerHTML = '<p style="color: #97A0AF;">変更履歴がありません</p>';
            }
        }

        modal.classList.add('active');
        document.body.style.overflow = 'hidden';
    }

    function closeModal() {
        const modal = document.getElementById('issue-modal');
        if (modal) modal.classList.remove('active');
        document.body.style.overflow = '';
    }

    function isDone(status) {
        const s = (status || '').toLowerCase();
        return s.includes('done') || s.includes('complete') || s.includes('closed') || s.includes('resolved');
    }

    function isInProgress(status) {
        const s = (status || '').toLowerCase();
        return s.includes('progress') || s.includes('active') || s.includes('review') || s.includes('testing');
    }

    function isBug(issueType) {
        return (issueType || '').toLowerCase().includes('bug');
    }

    function isStory(issueType) {
        return (issueType || '').toLowerCase().includes('story');
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

    function getPriorityOrder(priority) {
        const p = (priority || '').toLowerCase();
        if (p.includes('highest') || p.includes('blocker') || p.includes('critical')) return 5;
        if (p.includes('high') || p.includes('major')) return 4;
        if (p.includes('medium') || p.includes('normal')) return 3;
        if (p.includes('low') || p.includes('minor')) return 2;
        if (p.includes('lowest') || p.includes('trivial')) return 1;
        return 3;
    }

    function getPriorityColor(priority) {
        const p = (priority || '').toLowerCase();
        if (p.includes('highest') || p.includes('blocker') || p.includes('critical')) return '#FF5630';
        if (p.includes('high') || p.includes('major')) return '#FF7452';
        if (p.includes('medium') || p.includes('normal')) return '#FFAB00';
        if (p.includes('low') || p.includes('minor')) return '#36B37E';
        if (p.includes('lowest') || p.includes('trivial')) return '#97A0AF';
        return '#97A0AF';
    }

    function getAgeClass(age) {
        if (age <= 7) return 'age-new';
        if (age <= 30) return 'age-moderate';
        return 'age-old';
    }

    function getAgeLabel(age) {
        if (age <= 7) return '新規';
        if (age <= 30) return '中程度';
        return '古い';
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
