import { ComponentFixture, TestBed } from '@angular/core/testing';
import { IssuesComponent } from './issues.component';
import { API_SERVICE } from '../../api.provider';
import { Issue } from '../../generated/models';
import { of } from 'rxjs';

describe('IssuesComponent', () => {
  let component: IssuesComponent;
  let fixture: ComponentFixture<IssuesComponent>;

  // Mock API service
  const mockApiService = {
    projectsList: jest.fn().mockReturnValue(of({ projects: [] })),
    metadataGet: jest.fn().mockReturnValue(of({ metadata: { statuses: [], issueTypes: [] } })),
    issuesSearch: jest.fn().mockReturnValue(of({ issues: [], total: 0 })),
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [IssuesComponent],
      providers: [
        { provide: API_SERVICE, useValue: mockApiService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(IssuesComponent);
    component = fixture.componentInstance;
  });

  // Helper function to create mock issues
  function createMockIssue(overrides: Partial<Issue>): Issue {
    return {
      id: '1',
      key: 'TEST-1',
      projectKey: 'TEST',
      summary: 'Test issue',
      status: 'Open',
      priority: 'Medium',
      issueType: 'Task',
      labels: [],
      components: [],
      fixVersions: [],
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
      ...overrides,
    };
  }

  describe('Epic Grouping Logic', () => {
    it('should identify Epic issues by issueType', () => {
      const epicIssue = createMockIssue({
        key: 'EPIC-1',
        issueType: 'Epic',
        summary: 'Epic Summary',
      });

      const storyIssue = createMockIssue({
        key: 'STORY-1',
        issueType: 'Story',
        parentKey: 'EPIC-1',
      });

      component.issues.set([epicIssue, storyIssue]);
      component.statuses.set([{ name: 'Open', category: 'new', description: '' }]);
      component.groupBy.set('epic');

      const swimlanes = component.swimlanes();

      console.log('=== Test: identify Epic issues by issueType ===');
      console.log('Issues:', JSON.stringify([epicIssue, storyIssue], null, 2));
      console.log('Swimlanes:', JSON.stringify(swimlanes.map(s => ({ name: s.name, count: s.issueCount })), null, 2));

      // Check that Epic is identified
      const epicKeys = (component as any).epicKeys();
      console.log('Epic Keys:', Array.from(epicKeys));

      expect(epicKeys.has('EPIC-1')).toBe(true);
    });

    it('should group story under its epic parent', () => {
      const epicIssue = createMockIssue({
        key: 'EPIC-1',
        issueType: 'Epic',
        summary: 'Epic Summary',
      });

      const storyIssue = createMockIssue({
        key: 'STORY-1',
        issueType: 'Story',
        parentKey: 'EPIC-1',
        summary: 'Story under Epic',
      });

      component.issues.set([epicIssue, storyIssue]);
      component.statuses.set([{ name: 'Open', category: 'new', description: '' }]);
      component.groupBy.set('epic');

      const swimlanes = component.swimlanes();

      console.log('=== Test: group story under its epic parent ===');
      console.log('Swimlanes:', JSON.stringify(swimlanes.map(s => ({
        name: s.name,
        count: s.issueCount,
        issues: s.columns.flatMap(c => c.issues.map(i => i.key))
      })), null, 2));

      // Story should be grouped under Epic
      const epicSwimlane = swimlanes.find(s => s.name.includes('EPIC-1'));
      expect(epicSwimlane).toBeDefined();

      // Check findEpicForIssue
      const epicKey = (component as any).findEpicForIssue(storyIssue);
      console.log('findEpicForIssue result:', epicKey);
      expect(epicKey).toBe('EPIC-1');
    });

    it('should handle case-insensitive issueType matching', () => {
      // Test different casing of 'Epic'
      const epicLowercase = createMockIssue({
        key: 'EPIC-1',
        issueType: 'epic',  // lowercase
        summary: 'Lowercase Epic',
      });

      const epicUppercase = createMockIssue({
        key: 'EPIC-2',
        issueType: 'EPIC',  // uppercase
        summary: 'Uppercase Epic',
      });

      const epicMixed = createMockIssue({
        key: 'EPIC-3',
        issueType: 'Epic',  // mixed case
        summary: 'Mixed Case Epic',
      });

      component.issues.set([epicLowercase, epicUppercase, epicMixed]);
      component.groupBy.set('epic');

      const epicKeys = (component as any).epicKeys();

      console.log('=== Test: case-insensitive issueType matching ===');
      console.log('Epic Keys:', Array.from(epicKeys));

      expect(epicKeys.has('EPIC-1')).toBe(true);
      expect(epicKeys.has('EPIC-2')).toBe(true);
      expect(epicKeys.has('EPIC-3')).toBe(true);
    });

    it('should put issues without epic parent in No Epic group', () => {
      const orphanStory = createMockIssue({
        key: 'STORY-1',
        issueType: 'Story',
        parentKey: undefined,  // No parent
        summary: 'Orphan Story',
      });

      component.issues.set([orphanStory]);
      component.statuses.set([{ name: 'Open', category: 'new', description: '' }]);
      component.groupBy.set('epic');

      const swimlanes = component.swimlanes();

      console.log('=== Test: issues without epic parent in No Epic ===');
      console.log('Swimlanes:', JSON.stringify(swimlanes.map(s => ({
        name: s.name,
        count: s.issueCount
      })), null, 2));

      const noEpicSwimlane = swimlanes.find(s => s.name === 'No Epic');
      expect(noEpicSwimlane).toBeDefined();
      expect(noEpicSwimlane!.issueCount).toBe(1);
    });

    it('should handle sub-task -> story -> epic chain', () => {
      const epicIssue = createMockIssue({
        key: 'EPIC-1',
        issueType: 'Epic',
        summary: 'Epic',
      });

      const storyIssue = createMockIssue({
        key: 'STORY-1',
        issueType: 'Story',
        parentKey: 'EPIC-1',
        summary: 'Story under Epic',
      });

      const subtaskIssue = createMockIssue({
        key: 'SUBTASK-1',
        issueType: 'Sub-task',
        parentKey: 'STORY-1',  // Parent is Story, not Epic
        summary: 'Subtask under Story',
      });

      component.issues.set([epicIssue, storyIssue, subtaskIssue]);
      component.statuses.set([{ name: 'Open', category: 'new', description: '' }]);
      component.groupBy.set('epic');

      // Test findEpicForIssue for subtask
      const epicKeyForSubtask = (component as any).findEpicForIssue(subtaskIssue);

      console.log('=== Test: sub-task -> story -> epic chain ===');
      console.log('Epic Key for Subtask:', epicKeyForSubtask);

      const swimlanes = component.swimlanes();
      console.log('Swimlanes:', JSON.stringify(swimlanes.map(s => ({
        name: s.name,
        count: s.issueCount,
        issues: s.columns.flatMap(c => c.issues.map(i => i.key))
      })), null, 2));

      expect(epicKeyForSubtask).toBe('EPIC-1');
    });

    it('should handle story with non-epic parent going to No Epic', () => {
      // A story whose parentKey points to another story (not an epic)
      const story1 = createMockIssue({
        key: 'STORY-1',
        issueType: 'Story',
        parentKey: undefined,
        summary: 'Story 1',
      });

      const story2 = createMockIssue({
        key: 'STORY-2',
        issueType: 'Story',
        parentKey: 'STORY-1',  // Parent is another story, not an epic
        summary: 'Story 2 (child of Story 1)',
      });

      component.issues.set([story1, story2]);
      component.statuses.set([{ name: 'Open', category: 'new', description: '' }]);
      component.groupBy.set('epic');

      const epicKeyForStory2 = (component as any).findEpicForIssue(story2);

      console.log('=== Test: story with non-epic parent ===');
      console.log('Epic Key for Story2:', epicKeyForStory2);

      const swimlanes = component.swimlanes();
      console.log('Swimlanes:', JSON.stringify(swimlanes.map(s => ({
        name: s.name,
        count: s.issueCount
      })), null, 2));

      // Both stories should be in No Epic since no epic exists
      expect(epicKeyForStory2).toBeNull();
      const noEpicSwimlane = swimlanes.find(s => s.name === 'No Epic');
      expect(noEpicSwimlane).toBeDefined();
      expect(noEpicSwimlane!.issueCount).toBe(2);
    });

    it('should debug actual JIRA issueType values', () => {
      // Test with various possible issueType values from JIRA
      const possibleIssueTypes = [
        'Epic',
        'epic',
        'EPIC',
        'エピック',  // Japanese
        'Story',
        'Task',
        'Bug',
        'Sub-task',
      ];

      console.log('=== Debug: issueType lowercase matching ===');
      possibleIssueTypes.forEach(type => {
        const isEpic = type.toLowerCase() === 'epic';
        console.log(`issueType: "${type}" -> lowercase: "${type.toLowerCase()}" -> isEpic: ${isEpic}`);
      });

      // Note: Japanese 'エピック' will NOT match 'epic'
      expect('エピック'.toLowerCase()).not.toBe('epic');
      expect('Epic'.toLowerCase()).toBe('epic');
    });
  });
});
