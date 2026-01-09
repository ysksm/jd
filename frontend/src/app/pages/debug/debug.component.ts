import { Component, OnInit, signal, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import {
  Project,
  CreatedIssue,
  Transition,
  BulkTransitionResult,
  IssueTypeInfo,
  AiCreatedIssueInfo,
  AiFailedIssueInfo,
  AiGenerationStats,
} from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-debug',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './debug.component.html',
  styleUrl: './debug.component.scss',
})
export class DebugComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);

  // Status
  debugEnabled = signal(false);
  statusMessage = signal('');
  loading = signal(true);
  error = signal<string | null>(null);
  success = signal<string | null>(null);

  // Projects
  projects = signal<Project[]>([]);

  // Create Issues
  selectedProject = signal('');
  issueTypes = signal<IssueTypeInfo[]>([]);
  loadingIssueTypes = signal(false);
  issueCount = signal(1);
  issueType = signal('');
  summary = signal('[Debug] Test Issue');
  description = signal('');
  creatingIssues = signal(false);
  createdIssues = signal<CreatedIssue[]>([]);

  // Transitions
  transitionIssueKey = signal('');
  transitions = signal<Transition[]>([]);
  selectedTransition = signal('');
  loadingTransitions = signal(false);
  transitioning = signal(false);

  // Bulk Transition
  bulkIssueKeys = signal('');
  bulkTransitionId = signal('');
  bulkTransitioning = signal(false);
  bulkResults = signal<BulkTransitionResult[]>([]);

  // AI Test Data Generation
  aiConfigured = signal(false);
  aiStatusMessage = signal('');
  aiLoadingStatus = signal(false);
  aiGenerating = signal(false);
  aiMode = signal<'sprint' | 'epic' | 'bugs'>('sprint');
  aiProjectContext = signal('Web application with frontend and backend components');
  aiTeamSize = signal(4);
  aiSprintDuration = signal(14);
  aiApplyTransitions = signal(true);
  aiEpicTheme = signal('New Feature');
  aiBugCount = signal(5);
  aiUseFastModel = signal(false);
  aiCreatedIssues = signal<AiCreatedIssueInfo[]>([]);
  aiFailedIssues = signal<AiFailedIssueInfo[]>([]);
  aiStats = signal<AiGenerationStats | null>(null);

  ngOnInit(): void {
    this.loadStatus();
    this.loadProjects();
    this.loadAiStatus();
  }

  loadStatus(): void {
    this.loading.set(true);
    this.api.debugStatus({}).subscribe({
      next: (response) => {
        this.debugEnabled.set(response.enabled);
        this.statusMessage.set(response.message);
        this.loading.set(false);
      },
      error: (err) => {
        this.error.set('Failed to load debug status: ' + err);
        this.loading.set(false);
      },
    });
  }

  loadProjects(): void {
    this.api.projectsList({}).subscribe({
      next: (response) => {
        this.projects.set(response.projects);
        if (response.projects.length > 0) {
          this.selectedProject.set(response.projects[0].key);
          this.loadIssueTypes(response.projects[0].key);
        }
      },
      error: () => {
        // Ignore - projects may not be initialized
      },
    });
  }

  onProjectChange(projectKey: string): void {
    this.selectedProject.set(projectKey);
    this.loadIssueTypes(projectKey);
  }

  loadIssueTypes(projectKey: string): void {
    if (!projectKey) {
      this.issueTypes.set([]);
      return;
    }

    this.loadingIssueTypes.set(true);
    this.api.debugGetIssueTypes({ project: projectKey }).subscribe({
      next: (response) => {
        this.loadingIssueTypes.set(false);
        // Filter out subtasks for main issue creation
        const mainTypes = response.issueTypes.filter(t => !t.subtask);
        this.issueTypes.set(mainTypes);
        if (mainTypes.length > 0) {
          this.issueType.set(mainTypes[0].name);
        }
      },
      error: (err) => {
        this.loadingIssueTypes.set(false);
        // Fallback to common types if API fails
        console.warn('Failed to load issue types:', err);
        this.issueTypes.set([
          { name: 'Task', subtask: false },
          { name: 'Bug', subtask: false },
          { name: 'Story', subtask: false },
        ]);
        this.issueType.set('Task');
      },
    });
  }

  createIssues(): void {
    if (!this.selectedProject()) {
      this.error.set('Please select a project');
      return;
    }

    this.creatingIssues.set(true);
    this.error.set(null);
    this.success.set(null);
    this.createdIssues.set([]);

    this.api
      .debugCreateIssues({
        project: this.selectedProject(),
        count: this.issueCount(),
        issueType: this.issueType(),
        summary: this.summary(),
        description: this.description() || undefined,
      })
      .subscribe({
        next: (response) => {
          this.creatingIssues.set(false);
          if (response.success) {
            this.createdIssues.set(response.created);
            this.success.set(
              `Successfully created ${response.created.length} issue(s)`
            );
          } else {
            this.error.set(response.error || 'Failed to create issues');
          }
        },
        error: (err) => {
          this.creatingIssues.set(false);
          this.error.set('Failed to create issues: ' + err);
        },
      });
  }

  loadTransitions(): void {
    if (!this.transitionIssueKey()) {
      this.error.set('Please enter an issue key');
      return;
    }

    this.loadingTransitions.set(true);
    this.error.set(null);
    this.transitions.set([]);

    this.api
      .debugListTransitions({
        issueKey: this.transitionIssueKey(),
      })
      .subscribe({
        next: (response) => {
          this.loadingTransitions.set(false);
          this.transitions.set(response.transitions);
          if (response.transitions.length > 0) {
            this.selectedTransition.set(response.transitions[0].id);
          }
        },
        error: (err) => {
          this.loadingTransitions.set(false);
          this.error.set('Failed to load transitions: ' + err);
        },
      });
  }

  transitionIssue(): void {
    if (!this.transitionIssueKey() || !this.selectedTransition()) {
      this.error.set('Please select an issue and transition');
      return;
    }

    this.transitioning.set(true);
    this.error.set(null);
    this.success.set(null);

    this.api
      .debugTransitionIssue({
        issueKey: this.transitionIssueKey(),
        transitionId: this.selectedTransition(),
      })
      .subscribe({
        next: (response) => {
          this.transitioning.set(false);
          if (response.success) {
            this.success.set(
              `Successfully transitioned ${this.transitionIssueKey()}`
            );
            this.loadTransitions(); // Reload transitions
          } else {
            this.error.set(response.error || 'Failed to transition issue');
          }
        },
        error: (err) => {
          this.transitioning.set(false);
          this.error.set('Failed to transition issue: ' + err);
        },
      });
  }

  bulkTransition(): void {
    const issueKeys = this.bulkIssueKeys()
      .split(',')
      .map((k) => k.trim())
      .filter((k) => k);

    if (issueKeys.length === 0 || !this.bulkTransitionId()) {
      this.error.set('Please enter issue keys and transition ID');
      return;
    }

    this.bulkTransitioning.set(true);
    this.error.set(null);
    this.success.set(null);
    this.bulkResults.set([]);

    this.api
      .debugBulkTransition({
        issues: issueKeys,
        transitionId: this.bulkTransitionId(),
      })
      .subscribe({
        next: (response) => {
          this.bulkTransitioning.set(false);
          this.bulkResults.set(response.results);
          this.success.set(
            `Bulk transition complete: ${response.successCount} succeeded, ${response.failureCount} failed`
          );
        },
        error: (err) => {
          this.bulkTransitioning.set(false);
          this.error.set('Failed to bulk transition: ' + err);
        },
      });
  }

  // AI Test Data Generation Methods
  loadAiStatus(): void {
    this.aiLoadingStatus.set(true);
    this.api.debugAiStatus({}).subscribe({
      next: (response) => {
        this.aiLoadingStatus.set(false);
        this.aiConfigured.set(response.configured);
        this.aiStatusMessage.set(response.message);
      },
      error: (err) => {
        this.aiLoadingStatus.set(false);
        this.aiConfigured.set(false);
        this.aiStatusMessage.set('Failed to check AI status: ' + err);
      },
    });
  }

  onAiModeChange(mode: string): void {
    this.aiMode.set(mode as 'sprint' | 'epic' | 'bugs');
  }

  generateAiTestData(): void {
    if (!this.selectedProject()) {
      this.error.set('Please select a project');
      return;
    }

    if (!this.aiConfigured()) {
      this.error.set('AI is not configured. Please set ANTHROPIC_API_KEY environment variable.');
      return;
    }

    this.aiGenerating.set(true);
    this.error.set(null);
    this.success.set(null);
    this.aiCreatedIssues.set([]);
    this.aiFailedIssues.set([]);
    this.aiStats.set(null);

    this.api
      .debugAiGenerate({
        project: this.selectedProject(),
        mode: this.aiMode(),
        projectContext: this.aiProjectContext(),
        teamSize: this.aiTeamSize(),
        sprintDurationDays: this.aiSprintDuration(),
        applyTransitions: this.aiApplyTransitions(),
        epicTheme: this.aiEpicTheme(),
        bugCount: this.aiBugCount(),
        useFastModel: this.aiUseFastModel(),
      })
      .subscribe({
        next: (response) => {
          this.aiGenerating.set(false);
          if (response.success) {
            this.aiCreatedIssues.set(response.createdIssues);
            this.aiFailedIssues.set(response.failedIssues);
            this.aiStats.set(response.stats);
            this.success.set(
              `AI generated ${response.stats.totalGenerated} issues. ` +
              `Created: ${response.stats.successfullyCreated}, Failed: ${response.stats.failedToCreate}`
            );
          } else {
            this.error.set(response.error || 'AI generation failed');
          }
        },
        error: (err) => {
          this.aiGenerating.set(false);
          this.error.set('AI generation failed: ' + err);
        },
      });
  }

  clearMessages(): void {
    this.error.set(null);
    this.success.set(null);
  }
}
