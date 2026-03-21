import type { PropsWithChildren } from 'react';

type CardProps = PropsWithChildren<{
  className?: string;
}>;

export function Card({ children, className = '' }: CardProps) {
  return <section className={`card ${className}`.trim()}>{children}</section>;
}

export function CardEyebrow({ children }: PropsWithChildren) {
  return <p className="eyebrow">{children}</p>;
}

export function CardTitle({ children }: PropsWithChildren) {
  return <h1>{children}</h1>;
}

export function CardBody({ children }: PropsWithChildren) {
  return <>{children}</>;
}
