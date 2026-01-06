import { Component, Input, Output, EventEmitter, signal, computed, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Issue } from '../../../generated/models';

interface CalendarDay {
  date: Date;
  day: number;
  isCurrentMonth: boolean;
  isToday: boolean;
  issues: Issue[];
}

interface CalendarWeek {
  days: CalendarDay[];
}

@Component({
  selector: 'app-calendar',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './calendar.component.html',
  styleUrl: './calendar.component.scss'
})
export class CalendarComponent implements OnChanges {
  @Input() issues: Issue[] = [];
  @Output() issueClick = new EventEmitter<Issue>();

  currentDate = signal<Date>(new Date());
  weekDays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  // Map issues by due date for quick lookup
  private issuesByDate = computed<Map<string, Issue[]>>(() => {
    const map = new Map<string, Issue[]>();
    this.issues.forEach(issue => {
      if (issue.dueDate) {
        const dateKey = this.getDateKey(new Date(issue.dueDate));
        if (!map.has(dateKey)) {
          map.set(dateKey, []);
        }
        map.get(dateKey)!.push(issue);
      }
    });
    return map;
  });

  // Generate calendar weeks for current month
  weeks = computed<CalendarWeek[]>(() => {
    const current = this.currentDate();
    const year = current.getFullYear();
    const month = current.getMonth();

    const firstDayOfMonth = new Date(year, month, 1);
    const lastDayOfMonth = new Date(year, month + 1, 0);

    const startDate = new Date(firstDayOfMonth);
    startDate.setDate(startDate.getDate() - startDate.getDay());

    const endDate = new Date(lastDayOfMonth);
    endDate.setDate(endDate.getDate() + (6 - endDate.getDay()));

    const weeks: CalendarWeek[] = [];
    let currentWeek: CalendarDay[] = [];

    const today = new Date();
    today.setHours(0, 0, 0, 0);

    const issueMap = this.issuesByDate();

    for (let d = new Date(startDate); d <= endDate; d.setDate(d.getDate() + 1)) {
      const dateKey = this.getDateKey(d);
      const dayIssues = issueMap.get(dateKey) || [];

      currentWeek.push({
        date: new Date(d),
        day: d.getDate(),
        isCurrentMonth: d.getMonth() === month,
        isToday: d.getTime() === today.getTime(),
        issues: dayIssues
      });

      if (currentWeek.length === 7) {
        weeks.push({ days: currentWeek });
        currentWeek = [];
      }
    }

    return weeks;
  });

  currentMonthYear = computed<string>(() => {
    const current = this.currentDate();
    const year = current.getFullYear();
    const month = current.toLocaleString('default', { month: 'long' });
    return `${month} ${year}`;
  });

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['issues']) {
      // Trigger recomputation of issuesByDate
      this.currentDate.set(new Date(this.currentDate()));
    }
  }

  previousMonth(): void {
    const current = this.currentDate();
    const prev = new Date(current.getFullYear(), current.getMonth() - 1, 1);
    this.currentDate.set(prev);
  }

  nextMonth(): void {
    const current = this.currentDate();
    const next = new Date(current.getFullYear(), current.getMonth() + 1, 1);
    this.currentDate.set(next);
  }

  goToToday(): void {
    this.currentDate.set(new Date());
  }

  onIssueClick(issue: Issue, event: Event): void {
    event.stopPropagation();
    this.issueClick.emit(issue);
  }

  isOverdue(issue: Issue): boolean {
    if (!issue.dueDate) return false;
    const status = issue.status.toLowerCase();
    if (status === 'done' || status === 'closed' || status === 'resolved' || status === '完了') {
      return false;
    }
    const dueDate = new Date(issue.dueDate);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    return dueDate < today;
  }

  private getDateKey(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }
}
