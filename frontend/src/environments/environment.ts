/**
 * Base environment configuration
 * This file is replaced during build by using fileReplacements
 */
export const environment = {
  production: false,
  apiMode: 'http' as 'http' | 'tauri',
  apiBaseUrl: '/api',
};
