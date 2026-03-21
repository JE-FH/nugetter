import { Card, CardTitle } from './ui/Card';

type WatcherStatusCardProps = {
  status: string;
  pendingCount: number;
  error: string;
};

export function WatcherStatusCard({ status, pendingCount, error }: WatcherStatusCardProps) {
  return (
    <Card>
      <CardTitle>Stats</CardTitle>

      <div className="status-grid">
        <div>
          <span className="status-label">State</span>
          <p>{status}</p>
        </div>
        <div>
          <span className="status-label">Pending Updates</span>
          <p>{pendingCount}</p>
        </div>
      </div>

      {error ? <p className="error">{error}</p> : null}
    </Card>
  );
}
