import type { ComponentPropsWithoutRef, ElementType, ReactNode } from 'react';

import styles from './Button.module.scss';

type ButtonProps<TElement extends ElementType> = {
  as?: TElement;
  children: ReactNode;
} & Omit<ComponentPropsWithoutRef<TElement>, 'as' | 'children' | 'className'>;

export function Button<TElement extends ElementType = 'button'>({
  as,
  children,
  ...props
}: ButtonProps<TElement>) {
  const Component = as ?? 'button';

  return (
    <Component className={styles.button} {...props}>
      {children}
    </Component>
  );
}
