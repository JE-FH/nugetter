export const STORAGE_SETTINGS_KEY = 'nugetter.settings';

export const BACKEND_COMMANDS = {
  getSettings: 'get_settings',
  saveSettings: 'save_settings',
  processCopyRequest: 'process_copy_request',
  getLocalPackages: 'get_local_packages',
} as const;

export const WATCHER_EVENTS = {
  packageDetected: 'package-detected',
  status: 'watcher-status',
  error: 'watcher-error',
} as const;
