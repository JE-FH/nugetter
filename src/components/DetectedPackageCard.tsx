import type { PromptPayload } from '../types';
import { Button } from './ui/Button';
import { Card } from './ui/Card';

type DetectedPackageCardProps = {
  prompt: PromptPayload;
  isHandlingPrompt: boolean;
  onDecision: (approved: boolean) => void;
};

export function DetectedPackageCard({ prompt, isHandlingPrompt, onDecision }: DetectedPackageCardProps) {
  return (
    <Card className="prompt-card">
      <h2>New Package Detected</h2>
      <p>
        <strong>{prompt.packageId}</strong> was detected and will be repacked with version{' '}
        <strong>{prompt.nextVersion}</strong> before copy.
      </p>

      <dl>
        <dt>Source</dt>
        <dd>{prompt.sourcePath}</dd>

        <dt>Current version</dt>
        <dd>{prompt.currentVersion}</dd>

        <dt>New package</dt>
        <dd>{prompt.destinationFileName}</dd>

        <dt>Destination</dt>
        <dd>{prompt.destinationPath}</dd>
      </dl>

      <div className="prompt-actions">
        <Button
          type="button"
          variant="secondary"
          onClick={() => onDecision(false)}
          disabled={isHandlingPrompt}
        >
          Skip
        </Button>
        <Button
          type="button"
          variant="primary"
          onClick={() => onDecision(true)}
          disabled={isHandlingPrompt}
        >
          {isHandlingPrompt ? 'Working...' : 'Copy Package'}
        </Button>
      </div>
    </Card>
  );
}
