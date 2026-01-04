/**
 * Web (HTTP) environment configuration
 */
export const environment = {
  production: true,
  apiMode: 'http' as 'http' | 'tauri',
  apiBaseUrl: '/api',
};
