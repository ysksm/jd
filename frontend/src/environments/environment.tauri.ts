/**
 * Tauri environment configuration
 */
export const environment = {
  production: true,
  apiMode: 'tauri' as 'http' | 'tauri',
  apiBaseUrl: '', // Not used in Tauri mode
};
