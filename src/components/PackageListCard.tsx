import type { LocalPackageInfo, PromptPayload } from '../types';
import { Button } from './ui/Button';
import { Card, CardTitle } from './ui/Card';

type PackageListItem = {
  packageId: string;
  latestLocalVersion: string;
  pendingUpdate: PromptPayload | null;
};

type PackageListCardProps = {
  packages: PackageListItem[];
  isHandlingPrompt: boolean;
  onUpgrade: (packageId: string) => void;
  onDismiss: (packageId: string) => void;
};

export function buildPackageList(
  localPackages: LocalPackageInfo[],
  pendingByPackage: Record<string, PromptPayload>,
): PackageListItem[] {
  const map = new Map<string, PackageListItem>();

  for (const pkg of localPackages) {
    map.set(pkg.packageId, {
      packageId: pkg.packageId,
      latestLocalVersion: pkg.latestVersion,
      pendingUpdate: null,
    });
  }

  for (const pending of Object.values(pendingByPackage)) {
    const existing = map.get(pending.packageId);
    if (existing) {
      existing.pendingUpdate = pending;
      continue;
    }

    map.set(pending.packageId, {
      packageId: pending.packageId,
      latestLocalVersion: 'Not in local feed',
      pendingUpdate: pending,
    });
  }

  return [...map.values()].sort((a, b) => a.packageId.localeCompare(b.packageId));
}

export function PackageListCard({ packages, isHandlingPrompt, onUpgrade, onDismiss }: PackageListCardProps) {
  return (
    <Card className="package-card">
      <CardTitle>Local Packages</CardTitle>
      <p className="lead">Track your local feed and apply watcher-detected updates package-by-package.</p>

      {packages.length === 0 ? (
        <p className="empty-packages">No packages found yet. Save settings and build a C# project package to begin.</p>
      ) : (
        <div className="package-list">
          {packages.map((item) => (
            <article
              key={item.packageId}
              className="package-row"
            >
              <div>
                <h3>{item.packageId}</h3>
                <p>Latest local: {item.latestLocalVersion}</p>
                {item.pendingUpdate ? <p className="pending">Pending: {item.pendingUpdate.nextVersion}</p> : null}
              </div>

              {item.pendingUpdate ? (
                <div className="package-actions">
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={isHandlingPrompt}
                    onClick={() => onDismiss(item.packageId)}
                  >
                    Dismiss
                  </Button>
                  <Button
                    type="button"
                    variant="primary"
                    disabled={isHandlingPrompt}
                    onClick={() => onUpgrade(item.packageId)}
                  >
                    {isHandlingPrompt ? 'Applying...' : 'Upgrade'}
                  </Button>
                </div>
              ) : (
                <span className="up-to-date">Up to date</span>
              )}
            </article>
          ))}
        </div>
      )}
    </Card>
  );
}
