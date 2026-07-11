/**
 * Euclidean modulo. Unlike the native `%`, the result never goes negative for a positive divisor.
 *
 * @example
 * -1 % 3; // -1
 * mod(-1, 3); // 2
 */
export const mod = (value: number, divisor: number): number =>
  ((value % divisor) + divisor) % divisor;
