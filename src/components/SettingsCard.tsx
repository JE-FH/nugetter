import type { FormEvent } from 'react';
import { Button } from './ui/Button';
import { Card, CardTitle } from './ui/Card';
import { Form } from './ui/Form';
import { Input } from './ui/Input';
import { Lead } from './ui/Lead';

type SettingsCardProps = {
  watchPath: string;
  destinationPath: string;
  startVersion: string;
  isSubmitting: boolean;
  onWatchPathChange: (value: string) => void;
  onDestinationPathChange: (value: string) => void;
  onStartVersionChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
};

export function SettingsCard({
  watchPath,
  destinationPath,
  startVersion,
  isSubmitting,
  onWatchPathChange,
  onDestinationPathChange,
  onStartVersionChange,
  onSubmit,
}: SettingsCardProps) {
  return (
    <Card>
      <CardTitle>Configuration</CardTitle>
      <Lead>
        Watch a root folder containing multiple C# projects, prompt on new build packages, and publish incremented
        versions into your local feed.
      </Lead>

      <Form onSubmit={onSubmit}>
        <Input
          label="Folder to watch (contains C# projects)"
          id="watchPath"
          value={watchPath}
          onChange={(event) => onWatchPathChange(event.target.value)}
          placeholder="/home/user/dev/MySolution"
          required
        />
        <Input
          label="Local NuGet feed folder"
          id="destinationPath"
          value={destinationPath}
          onChange={(event) => onDestinationPathChange(event.target.value)}
          placeholder="/home/user/local-feed"
          required
        />
        <Input
          label="Starting version for first local copy"
          id="startVersion"
          value={startVersion}
          onChange={(event) => onStartVersionChange(event.target.value)}
          placeholder="1.0.0"
          required
        />
        <Button
          type="submit"
          disabled={isSubmitting}
          variant="primary"
        >
          {isSubmitting ? 'Saving...' : 'Save Settings And Start Watching'}
        </Button>
      </Form>
    </Card>
  );
}
