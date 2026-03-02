// Re-export upstream hooks. Note: useTable is wrapped below to extract rows from tuple.
export { SpacetimeDBProvider, useReducer, useSpacetimeDB } from 'spacetimedb/react';

import { useState, useCallback } from 'react';
import { useTable as upstreamUseTable, useReducer } from 'spacetimedb/react';

/**
 * useTable — wraps upstream useTable which returns [rows, isReady] tuple.
 * ReScript binding types this as array<'row>, so we extract only the rows.
 * 
 * If you need the isReady flag, use useTableState instead.
 */
export function useTable(query, callbacks) {
  const [rows] = upstreamUseTable(query, callbacks);
  return rows;
}

/**
 * useTableWith — same as useTable but with explicit callbacks.
 * Upstream returns [rows, isReady] tuple, we extract only rows.
 */
export function useTableWith(query, callbacks) {
  const [rows] = upstreamUseTable(query, callbacks);
  return rows;
}

/**
 * useTableState — wraps useTable and surfaces the isReady flag as a
 * ReScript variant: Loading | Ready(array<'row>).
 *
 * Upstream useTable returns [rows, isReady].  We convert to the variant
 * encoding that ReScript v12 expects:
 *   Loading      → "Loading"                (string tag, no payload)
 *   Ready(rows)  → { TAG: "Ready", _0: rows }
 */
export function useTableState(query) {
  const [rows, isReady] = upstreamUseTable(query);
  if (isReady) {
    return { TAG: "Ready", _0: rows };
  }
  return "Loading";
}

/**
 * useMutation — wraps useReducer with pending/error state tracking.
 *
 * Returns { call, isPending, error, reset } matching ReScript type:
 *   type mutationState<'args, 'error> = {
 *     call: 'args => unit,
 *     isPending: bool,
 *     error: option<'error>,
 *     reset: unit => unit,
 *   }
 *
 * ReScript option encoding: None → undefined, Some(v) → v.
 */
export function useMutation(reducerDef) {
  const callReducer = useReducer(reducerDef);
  const [isPending, setIsPending] = useState(false);
  const [error, setError] = useState(undefined);

  const call = useCallback(
    (args) => {
      setIsPending(true);
      setError(undefined);
      callReducer(args).then(
        () => { setIsPending(false); },
        (err) => { setIsPending(false); setError(err); },
      );
    },
    [callReducer],
  );

  const reset = useCallback(() => {
    setIsPending(false);
    setError(undefined);
  }, []);

  return { call, isPending, error, reset };
}
