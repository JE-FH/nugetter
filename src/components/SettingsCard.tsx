import type { FormEvent } from 'react';
import { Button } from './ui/Button';
import { Card } from './ui/Card';

type SettingsCardProps = {
  watchPath: string;
  destinationPath: string;
  status: string;
  queuedPromptCount: number;
  error: string;
  isSubmitting: boolean;
  onWatchPathChange: (value: string) => void;
  onDestinationPathChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
};

export function SettingsCard({
  watchPath,
  destinationPath,
  status,
  queuedPromptCount,
  error,
  isSubmitting,
  onWatchPathChange,
  onDestinationPathChange,
  onSubmit,
}: SettingsCardProps) {
  return (
    <Card>
      <p className="eyebrow">Local NuGet Flow</p>
      <h1>Nugetter Watcher</h1>
      <p className="lead">
        Watch a root folder containing multiple C# projects, prompt on new build packages, and publish incremented
        versions into your local feed.
      </p>

      <form
        className="form"
        onSubmit={onSubmit}
      >
        <label htmlFor="watchPath">Folder to watch (contains C# projects)</label>
        <input
          id="watchPath"
          value={watchPath}
          onChange={(event) => onWatchPathChange(event.target.value)}
          placeholder="/home/user/dev/MySolution"
          required
        />

        <label htmlFor="destinationPath">Local NuGet feed folder</label>
        <input
          id="destinationPath"
          value={destinationPath}
          onChange={(event) => onDestinationPathChange(event.target.value)}
          placeholder="/home/user/local-feed"
          required
        />

        <Button
          type="submit"
          disabled={isSubmitting}
          variant="primary"
        >
          {isSubmitting ? 'Saving...' : 'Save Settings And Start Watching'}
        </Button>
      </form>

      <div className="status-grid">
        <div>
          <span className="status-label">Watcher</span>
          <p>{status}</p>
        </div>
        <div>
          <span className="status-label">Queued Prompts</span>
          <p>{queuedPromptCount}</p>
        </div>
      </div>

      {error ? <p className="error">{error}</p> : null}
    </Card>
  );
}
