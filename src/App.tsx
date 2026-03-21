import './style.css';
import { useEffect, useMemo, useState, type FormEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DetectedPackageCard } from './components/DetectedPackageCard';
import { SettingsCard } from './components/SettingsCard';
import { BACKEND_COMMANDS, STORAGE_SETTINGS_KEY, WATCHER_EVENTS } from './constants';
import type { PromptPayload, WatchSettings } from './types';

function App() {
  const [watchPath, setWatchPath] = useState('');
  const [destinationPath, setDestinationPath] = useState('');
  const [status, setStatus] = useState('Idle');
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isHandlingPrompt, setIsHandlingPrompt] = useState(false);
  const [promptQueue, setPromptQueue] = useState<PromptPayload[]>([]);

  const applySettings = (settings: Partial<WatchSettings>) => {
    setWatchPath(settings.watchPath ?? '');
    setDestinationPath(settings.destinationPath ?? '');
  };

  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_SETTINGS_KEY);
    if (saved) {
      try {
        const parsed = JSON.parse(saved) as WatchSettings;
        applySettings(parsed);
      } catch {
        // Ignore invalid localStorage shape and keep defaults.
      }
    }

    invoke<WatchSettings | null>(BACKEND_COMMANDS.getSettings)
      .then((settings) => {
        if (!settings) {
          return;
        }
        applySettings(settings);
      })
      .catch(() => {
        // Ignore backend state load failures on startup.
      });
  }, []);

  useEffect(() => {
    let unlistenDetected: null | (() => void) = null;
    let unlistenStatus: null | (() => void) = null;
    let unlistenError: null | (() => void) = null;

    const setupListeners = async () => {
      unlistenDetected = await listen<PromptPayload>(WATCHER_EVENTS.packageDetected, (event) => {
        setPromptQueue((current) => [...current, event.payload]);
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

  const currentPrompt = useMemo(() => promptQueue[0], [promptQueue]);

  const submitSettings = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError('');
    setIsSubmitting(true);

    const settings: WatchSettings = {
      watchPath: watchPath.trim(),
      destinationPath: destinationPath.trim(),
    };

    try {
      const message = await invoke<string>(BACKEND_COMMANDS.saveSettings, { settings });
      localStorage.setItem(STORAGE_SETTINGS_KEY, JSON.stringify(settings));
      setStatus(message);
    } catch (submitError) {
      setError(String(submitError));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePromptDecision = async (approved: boolean) => {
    if (!currentPrompt) {
      return;
    }

    const promptToHandle = currentPrompt;

    // Optimistically remove the prompt so the modal disappears immediately.
    setPromptQueue((current) => current.filter((item) => item.requestId !== promptToHandle.requestId));

    setIsHandlingPrompt(true);
    setError('');
    setStatus(approved ? 'Copying approved package...' : 'Skipping package...');

    try {
      const result = await invoke<string>(BACKEND_COMMANDS.processCopyRequest, {
        requestId: promptToHandle.requestId,
        approved,
      });
      setStatus(result);
    } catch (processError) {
      setError(String(processError));
    } finally {
      setIsHandlingPrompt(false);
    }
  };

  return (
    <main className="shell">
      <SettingsCard
        watchPath={watchPath}
        destinationPath={destinationPath}
        status={status}
        queuedPromptCount={promptQueue.length}
        error={error}
        isSubmitting={isSubmitting}
        onWatchPathChange={setWatchPath}
        onDestinationPathChange={setDestinationPath}
        onSubmit={submitSettings}
      />

      {currentPrompt ? (
        <DetectedPackageCard
          prompt={currentPrompt}
          isHandlingPrompt={isHandlingPrompt}
          onDecision={handlePromptDecision}
        />
      ) : null}
    </main>
  );
}

export default App;
