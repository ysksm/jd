/**
 * API Service Provider
 *
 * Provides the appropriate API service based on the environment configuration.
 * - Web mode: Uses HttpClient-based ApiService
 * - Tauri mode: Uses Tauri invoke-based TauriApiService
 */
import { InjectionToken, Provider } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { environment } from '../environments/environment';
import { ApiService, TauriApiService } from './generated';

/**
 * Abstract type representing the API service interface.
 * Both ApiService and TauriApiService implement the same methods.
 */
export type IApiService = ApiService | TauriApiService;

/**
 * Injection token for the API service.
 * Components should inject this token instead of concrete service classes.
 */
export const API_SERVICE = new InjectionToken<IApiService>('API_SERVICE');

/**
 * Factory function that creates the appropriate API service based on environment.
 */
export function apiServiceFactory(http: HttpClient): IApiService {
  if (environment.apiMode === 'tauri') {
    return new TauriApiService();
  }
  return new ApiService(http);
}

/**
 * Provider configuration for the API service.
 * Add this to your application's providers array.
 */
export const API_SERVICE_PROVIDER: Provider = {
  provide: API_SERVICE,
  useFactory: apiServiceFactory,
  deps: [HttpClient],
};
