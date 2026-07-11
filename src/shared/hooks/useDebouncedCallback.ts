import { useCallback, useEffect, useMemo, useRef } from 'react';
import { debounce, type DebouncedFunc } from 'lodash-es';

export interface DebouncedCallback<Args extends unknown[]> {
  /** Планирует вызов, перезапуская задержку, если вызов уже ожидает выполнения. */
  run: (...args: Args) => void;
  /** Отменяет вызов, который всё ещё ожидает истечения задержки. */
  cancel: () => void;
  /** Выполняет ожидающий вызов немедленно, не дожидаясь истечения задержки. Ничего не делает, если вызовов не ожидается. */
  flush: () => void;
}

/**
 * Дебаунсит `callback` и предоставляет `run`, `cancel` и `flush`.
 *
 * Возвращаемый объект и его методы сохраняют ссылочную стабильность на протяжении всего времени
 * жизни компонента, поэтому их безопасно использовать в массиве зависимостей. `callback` может
 * пересоздаваться на каждом рендере без перезапуска задержки и без потери ожидающего вызова,
 * поскольку он читается через ref в момент вызова. Ожидающий вызов сбрасывается при размонтировании
 * и при каждом изменении `delayMs`.
 *
 * @param callback Вызывается по истечении задержки с аргументами последнего вызова `run`.
 * @param delayMs Количество миллисекунд ожидания после последнего вызова `run`.
 */
export const useDebouncedCallback = <Args extends unknown[]>(
  callback: (...args: Args) => void,
  delayMs: number,
): DebouncedCallback<Args> => {
  const callbackRef = useRef(callback);
  const debouncedRef = useRef<DebouncedFunc<(...args: Args) => void>>(undefined);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  // Создаётся вне рендера, чтобы экземпляр debounced переживал повторные рендеры без изменений. В результате
  // `run` начинает работать только после монтирования компонента, что и требуется обработчикам событий.
  useEffect(() => {
    const debounced = debounce((...args: Args) => {
      callbackRef.current(...args);
    }, delayMs);

    debouncedRef.current = debounced;

    return () => {
      debounced.cancel();
      debouncedRef.current = undefined;
    };
  }, [delayMs]);

  const run = useCallback((...args: Args) => {
    debouncedRef.current?.(...args);
  }, []);

  const cancel = useCallback(() => {
    debouncedRef.current?.cancel();
  }, []);

  const flush = useCallback(() => {
    debouncedRef.current?.flush();
  }, []);

  return useMemo(() => ({ cancel, flush, run }), [cancel, flush, run]);
};
