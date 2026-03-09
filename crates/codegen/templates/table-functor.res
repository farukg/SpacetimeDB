{{self.header}}

{{self.sibling_opens}}

// ── TABLE module type ─────────────────────────────────────────────────────────
// Every per-table file satisfies this by declaring `type t`, `type handle`,
// and the `@send external` bindings that the JS SDK provides.
module type TABLE = {
  type t
  type handle
  let iter: handle => Iterator.t<t>
  let onInsert: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit
  let removeOnInsert: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit
  let onUpdate: (handle, @uncurry (Sdk.eventCtx, t, t) => unit) => unit
  let removeOnUpdate: (handle, @uncurry (Sdk.eventCtx, t, t) => unit) => unit
  let onDelete: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit
  let removeOnDelete: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit
}

// ── Make functor ──────────────────────────────────────────────────────────────
// Produces: type event, let subscribe, module MakeStream
module Make = (T: TABLE) => {
  let rows = (handle: T.handle): array<T.t> => handle->T.iter->Array.fromIterator
  type event =
    | Inserted({row: T.t})
    | Updated({prev: T.t, next: T.t})
    | Deleted({row: T.t})

  let watchInsert = (handle: T.handle, callback: unit => unit) => {
    let insertCb = (_ctx: Sdk.eventCtx, _row: T.t) => callback()
    T.onInsert(handle, insertCb)
    () => T.removeOnInsert(handle, insertCb)
  }

  let watchUpdate = (handle: T.handle, callback: unit => unit) => {
    let updateCb = (_ctx: Sdk.eventCtx, _prev: T.t, _next: T.t) => callback()
    T.onUpdate(handle, updateCb)
    () => T.removeOnUpdate(handle, updateCb)
  }

  let watchDelete = (handle: T.handle, callback: unit => unit) => {
    let deleteCb = (_ctx: Sdk.eventCtx, _row: T.t) => callback()
    T.onDelete(handle, deleteCb)
    () => T.removeOnDelete(handle, deleteCb)
  }

  let onChange = (handle: T.handle, callback: unit => unit) => {
    let removeInsert = watchInsert(handle, callback)
    let removeUpdate = watchUpdate(handle, callback)
    let removeDelete = watchDelete(handle, callback)
    () => {
      removeInsert()
      removeUpdate()
      removeDelete()
    }
  }

  let subscribe = (handle: T.handle, handler: event => unit): (unit => unit) => {
    let insertCb = (_ctx: Sdk.eventCtx, row: T.t) => handler(Inserted({row: row}))
    let updateCb = (_ctx: Sdk.eventCtx, prev: T.t, next: T.t) => handler(Updated({prev: prev, next: next}))
    let deleteCb = (_ctx: Sdk.eventCtx, row: T.t) => handler(Deleted({row: row}))
    T.onInsert(handle, insertCb)
    T.onUpdate(handle, updateCb)
    T.onDelete(handle, deleteCb)
    () => {
      T.removeOnInsert(handle, insertCb)
      T.removeOnUpdate(handle, updateCb)
      T.removeOnDelete(handle, deleteCb)
    }
  }
%% if self.has_observer {

  module MakeStream = (O: Async.OBSERVER) => {
    let observe = (handle: T.handle): O.stream<event> => {
      let ins = O.fromCallback(emit => {
        let cb = (_ctx: Sdk.eventCtx, row: T.t) => emit(Inserted({row: row}))
        T.onInsert(handle, cb)
        () => T.removeOnInsert(handle, cb)
      })
      let upd = O.fromCallback(emit => {
        let cb = (_ctx: Sdk.eventCtx, prev: T.t, next: T.t) => emit(Updated({prev: prev, next: next}))
        T.onUpdate(handle, cb)
        () => T.removeOnUpdate(handle, cb)
      })
      let del = O.fromCallback(emit => {
        let cb = (_ctx: Sdk.eventCtx, row: T.t) => emit(Deleted({row: row}))
        T.onDelete(handle, cb)
        () => T.removeOnDelete(handle, cb)
      })
      O.merge([ins, upd, del])
    }

    let observeWithCtx = (handle: T.handle): O.stream<(Sdk.eventCtx, event)> => {
      let ins = O.fromCallback(emit => {
        let cb = (ctx: Sdk.eventCtx, row: T.t) => emit((ctx, Inserted({row: row})))
        T.onInsert(handle, cb)
        () => T.removeOnInsert(handle, cb)
      })
      let upd = O.fromCallback(emit => {
        let cb = (ctx: Sdk.eventCtx, prev: T.t, next: T.t) => emit((ctx, Updated({prev: prev, next: next})))
        T.onUpdate(handle, cb)
        () => T.removeOnUpdate(handle, cb)
      })
      let del = O.fromCallback(emit => {
        let cb = (ctx: Sdk.eventCtx, row: T.t) => emit((ctx, Deleted({row: row})))
        T.onDelete(handle, cb)
        () => T.removeOnDelete(handle, cb)
      })
      O.merge([ins, upd, del])
    }
  }
%% }
}
