import { Routes } from '@angular/router';
import { DashboardComponent } from './pages/dashboard/dashboard.component';
import { ProjectsComponent } from './pages/projects/projects.component';
import { IssuesComponent } from './pages/issues/issues.component';
import { QueryComponent } from './pages/query/query.component';
import { VisualizationComponent } from './pages/visualization/visualization.component';
import { SyncComponent } from './pages/sync/sync.component';
import { SettingsComponent } from './pages/settings/settings.component';

export const routes: Routes = [
  { path: '', redirectTo: '/dashboard', pathMatch: 'full' },
  { path: 'dashboard', component: DashboardComponent },
  { path: 'projects', component: ProjectsComponent },
  { path: 'issues', component: IssuesComponent },
  { path: 'query', component: QueryComponent },
  { path: 'visualization', component: VisualizationComponent },
  { path: 'sync', component: SyncComponent },
  { path: 'settings', component: SettingsComponent },
];
