import type { ComponentPropsWithoutRef, ElementType, ReactElement, ReactNode } from 'react';

import styles from './Button.module.scss';

interface ButtonProps<TElement extends ElementType> {
  as?: TElement;
  children: ReactNode;
}

type PolymorphicButtonProps<TElement extends ElementType> = ButtonProps<TElement> &
  Omit<ComponentPropsWithoutRef<TElement>, 'as' | 'children' | 'className'>;

type ButtonComponent = <TElement extends ElementType = 'button'>(
  props: PolymorphicButtonProps<TElement>,
) => ReactElement | null;

const Button: ButtonComponent = ({ as, children, ...props }) => {
  const Component = as ?? 'button';

  return (
    <Component className={styles.button} {...props}>
      {children}
    </Component>
  );
};

export default Button;
