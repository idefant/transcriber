import { mod } from './mod';

/**
 * Cyclically shifts elements to the right, wrapping the tail around to the front. A negative
 * `step` shifts to the left, and a step larger than the array wraps around.
 *
 * @example
 * rotate([1, 2, 3], 1); // [3, 1, 2]
 * rotate([1, 2, 3], -1); // [2, 3, 1]
 */
export const rotate = <T>(array: readonly T[], step: number): T[] => {
  if (array.length === 0) {
    return [];
  }

  const pivot = array.length - mod(step, array.length);

  return [...array.slice(pivot), ...array.slice(0, pivot)];
};
