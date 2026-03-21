import type { PropsWithChildren } from 'react';

type Props = React.FormHTMLAttributes<HTMLFormElement> & PropsWithChildren;

export function Form({ children, ...otherProps }: Props) {
  return (
    <form
      className="form"
      {...otherProps}
    >
      {children}
    </form>
  );
}
