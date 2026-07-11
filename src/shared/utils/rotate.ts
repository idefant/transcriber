import { mod } from './mod';

/**
 * Циклически сдвигает элементы вправо, перенося конец массива в начало. Отрицательный
 * `step` сдвигает влево, а шаг больше длины массива зацикливается.
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
