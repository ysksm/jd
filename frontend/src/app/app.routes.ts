import { Routes } from '@angular/router';
import { DashboardComponent } from './pages/dashboard/dashboard.component';
import { ProjectsComponent } from './pages/projects/projects.component';
import { ProjectDetailComponent } from './pages/project-detail/project-detail.component';
import { SyncComponent } from './pages/sync/sync.component';
import { SettingsComponent } from './pages/settings/settings.component';
import { DebugComponent } from './pages/debug/debug.component';

export const routes: Routes = [
  { path: '', redirectTo: '/dashboard', pathMatch: 'full' },
  { path: 'dashboard', component: DashboardComponent },
  { path: 'projects', component: ProjectsComponent },
  {
    path: 'projects/:projectKey',
    component: ProjectDetailComponent,
    children: [
      { path: '', redirectTo: 'overview', pathMatch: 'full' },
      { path: 'overview', component: ProjectDetailComponent },
      { path: 'issues', component: ProjectDetailComponent },
      { path: 'query', component: ProjectDetailComponent },
      { path: 'charts', component: ProjectDetailComponent },
    ]
  },
  { path: 'sync', component: SyncComponent },
  { path: 'settings', component: SettingsComponent },
  { path: 'debug', component: DebugComponent },
];
