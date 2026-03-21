import type { PropsWithChildren } from 'react';

export function Lead({ children }: PropsWithChildren) {
  return <p className="lead">{children}</p>;
}
