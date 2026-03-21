import './style.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { buildPackageList, PackageListCard } from './components/PackageListCard';
import { SettingsCard } from './components/SettingsCard';
import { WatcherStatusCard } from './components/WatcherStatusCard';
import { BACKEND_COMMANDS, STORAGE_SETTINGS_KEY, WATCHER_EVENTS } from './constants';
import type { LocalPackageInfo, PromptPayload, WatchSettings } from './types';

function App() {
  const [watchPath, setWatchPath] = useState('');
  const [destinationPath, setDestinationPath] = useState('');
  const [startVersion, setStartVersion] = useState('1.0.0');
  const [status, setStatus] = useState('Idle');
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isHandlingPrompt, setIsHandlingPrompt] = useState(false);
  const [localPackages, setLocalPackages] = useState<LocalPackageInfo[]>([]);
  const [pendingByPackage, setPendingByPackage] = useState<Record<string, PromptPayload>>({});

  const applySettings = useCallback((settings: Partial<WatchSettings>) => {
    setWatchPath(settings.watchPath ?? '');
    setDestinationPath(settings.destinationPath ?? '');
    setStartVersion(settings.startVersion?.trim() || '1.0.0');
  }, []);

  const loadLocalPackages = useCallback(async () => {
    try {
      const packages = await invoke<LocalPackageInfo[]>(BACKEND_COMMANDS.getLocalPackages);
      setLocalPackages(packages);
    } catch {
      // Ignore package list loading errors when settings are not configured yet.
    }
  }, []);

  useEffect(() => {
    const parseSavedSettings = (): WatchSettings | null => {
      const raw = localStorage.getItem(STORAGE_SETTINGS_KEY);
      if (!raw) {
        return null;
      }

      try {
        return JSON.parse(raw) as WatchSettings;
      } catch {
        return null;
      }
    };

    const normalizeSettings = (settings: WatchSettings): WatchSettings => ({
      watchPath: settings.watchPath.trim(),
      destinationPath: settings.destinationPath.trim(),
      startVersion: settings.startVersion?.trim() || '1.0.0',
    });

    const bootstrap = async () => {
      const localSettings = parseSavedSettings();
      if (localSettings) {
        applySettings(localSettings);
      }

      let startupSettings = localSettings;
      try {
        const backendSettings = await invoke<WatchSettings | null>(BACKEND_COMMANDS.getSettings);
        if (backendSettings) {
          startupSettings = backendSettings;
          applySettings(backendSettings);
        }
      } catch {
        // Ignore backend state load failures on startup.
      }

      if (!startupSettings) {
        return;
      }

      const normalizedSettings = normalizeSettings(startupSettings);
      if (!normalizedSettings.watchPath || !normalizedSettings.destinationPath) {
        return;
      }

      try {
        const message = await invoke<string>(BACKEND_COMMANDS.saveSettings, {
          settings: normalizedSettings,
        });
        localStorage.setItem(STORAGE_SETTINGS_KEY, JSON.stringify(normalizedSettings));
        setStatus(message);
        await loadLocalPackages();
      } catch (startupError) {
        setError(String(startupError));
      }
    };

    void bootstrap();
  }, [applySettings, loadLocalPackages]);

  useEffect(() => {
    let unlistenDetected: null | (() => void) = null;
    let unlistenStatus: null | (() => void) = null;
    let unlistenError: null | (() => void) = null;

    const setupListeners = async () => {
      unlistenDetected = await listen<PromptPayload>(WATCHER_EVENTS.packageDetected, (event) => {
        setPendingByPackage((current) => ({ ...current, [event.payload.packageId]: event.payload }));
      });

      unlistenStatus = await listen<string>(WATCHER_EVENTS.status, (event) => {
        setStatus(event.payload);
      });

      unlistenError = await listen<string>(WATCHER_EVENTS.error, (event) => {
        setError(event.payload);
      });
    };

    setupListeners();

    return () => {
      unlistenDetected?.();
      unlistenStatus?.();
      unlistenError?.();
    };
  }, []);

  const packageList = useMemo(
    () => buildPackageList(localPackages, pendingByPackage),
    [localPackages, pendingByPackage],
  );

  const submitSettings = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError('');
    setIsSubmitting(true);

    const settings: WatchSettings = {
      watchPath: watchPath.trim(),
      destinationPath: destinationPath.trim(),
      startVersion: startVersion.trim() || '1.0.0',
    };

    try {
      const message = await invoke<string>(BACKEND_COMMANDS.saveSettings, { settings });
      localStorage.setItem(STORAGE_SETTINGS_KEY, JSON.stringify(settings));
      setStatus(message);
      await loadLocalPackages();
    } catch (submitError) {
      setError(String(submitError));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleUpgradePackage = async (packageId: string) => {
    const pending = pendingByPackage[packageId];
    if (!pending) {
      return;
    }
    setPendingByPackage((current) => {
      const next = { ...current };
      delete next[packageId];
      return next;
    });

    setIsHandlingPrompt(true);
    setError('');
    setStatus(`Upgrading ${packageId}...`);

    try {
      const result = await invoke<string>(BACKEND_COMMANDS.processCopyRequest, {
        requestId: pending.requestId,
        approved: true,
      });
      setStatus(result);
      await loadLocalPackages();
    } catch (processError) {
      setError(String(processError));
    } finally {
      setIsHandlingPrompt(false);
    }
  };

  const handleDismissUpdate = async (packageId: string) => {
    const pending = pendingByPackage[packageId];
    if (!pending) {
      return;
    }

    setPendingByPackage((current) => {
      const next = { ...current };
      delete next[packageId];
      return next;
    });

    try {
      await invoke<string>(BACKEND_COMMANDS.processCopyRequest, {
        requestId: pending.requestId,
        approved: false,
      });
      setStatus(`Dismissed update for ${packageId}`);
    } catch (dismissError) {
      setError(String(dismissError));
    }
  };

  return (
    <main className="shell">
      <PackageListCard
        packages={packageList}
        isHandlingPrompt={isHandlingPrompt}
        onUpgrade={handleUpgradePackage}
        onDismiss={handleDismissUpdate}
      />

      <section className="side-column">
        <WatcherStatusCard
          status={status}
          pendingCount={Object.keys(pendingByPackage).length}
          error={error}
        />

        <SettingsCard
          watchPath={watchPath}
          destinationPath={destinationPath}
          startVersion={startVersion}
          isSubmitting={isSubmitting}
          onWatchPathChange={setWatchPath}
          onDestinationPathChange={setDestinationPath}
          onStartVersionChange={setStartVersion}
          onSubmit={submitSettings}
        />
      </section>
    </main>
  );
}

export default App;
