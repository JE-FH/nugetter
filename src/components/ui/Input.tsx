import { type ReactNode, useMemo } from 'react';

type Props = React.InputHTMLAttributes<HTMLInputElement> & {
  label: string | ReactNode | null;
};

export function Input({ label, ...props }: Props) {
  const labelElement = useMemo(() => {
    if (typeof label === 'string') {
      return <label htmlFor={props.id}>{label}</label>;
    } else if (label == null) {
      return null;
    } else {
      return label;
    }
  }, [label, props.id]);

  return (
    <>
      {labelElement}
      <input {...props} />
    </>
  );
}
