import { useCallback, useEffect, useMemo, useRef } from 'react';
import { debounce, type DebouncedFunc } from 'lodash-es';

export interface DebouncedCallback<Args extends unknown[]> {
  /** Schedules a call, restarting the delay if one is already waiting. */
  run: (...args: Args) => void;
  /** Drops a call that is still waiting for the delay to elapse. */
  cancel: () => void;
  /** Runs a waiting call right now instead of waiting out the delay. Does nothing when idle. */
  flush: () => void;
}

/**
 * Debounces `callback` and exposes `run`, `cancel`, and `flush`.
 *
 * The returned object and its methods are referentially stable for the lifetime of the component,
 * so they are safe to use in a dependency array. `callback` may be recreated on every render
 * without restarting the delay or dropping a waiting call, because it is read through a ref at
 * invoke time. A waiting call is dropped on unmount and whenever `delayMs` changes.
 *
 * @param callback Invoked once the delay elapses, with the arguments of the latest `run` call.
 * @param delayMs Milliseconds to wait after the last `run` call.
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

  // Built outside render so the debounced instance survives re-renders untouched. As a result
  // `run` only starts working once the component is mounted, which is what event handlers need.
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
