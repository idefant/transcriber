/**
 * Евклидов остаток от деления. В отличие от нативного `%`, результат никогда не становится отрицательным
 * при положительном делителе.
 *
 * @example
 * -1 % 3; // -1
 * mod(-1, 3); // 2
 */
export const mod = (value: number, divisor: number): number =>
  ((value % divisor) + divisor) % divisor;
