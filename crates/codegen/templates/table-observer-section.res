
// ── Observer mode ─────────────────────────────────────────────────────────────
module MakeStream = (O: Stdb__Async.OBSERVER) => {
  let observe = (handle: handle): O.stream<event> => {
    let ins = O.fromCallback(emit => {
      let cb = (_ctx: {{self.sdk_module}}.eventCtx, row: t) => emit(Inserted({row: row}))
      handle->onInsert(cb)
      () => handle->removeOnInsert(cb)
    })
    let upd = O.fromCallback(emit => {
      let cb = (_ctx: {{self.sdk_module}}.eventCtx, prev: t, next: t) => emit(Updated({prev: prev, next: next}))
      handle->onUpdate(cb)
      () => handle->removeOnUpdate(cb)
    })
    let del = O.fromCallback(emit => {
      let cb = (_ctx: {{self.sdk_module}}.eventCtx, row: t) => emit(Deleted({row: row}))
      handle->onDelete(cb)
      () => handle->removeOnDelete(cb)
    })
    O.merge([ins, upd, del])
  }

  let observeWithCtx = (handle: handle): O.stream<({{self.sdk_module}}.eventCtx, event)> => {
    let ins = O.fromCallback(emit => {
      let cb = (ctx: {{self.sdk_module}}.eventCtx, row: t) => emit((ctx, Inserted({row: row})))
      handle->onInsert(cb)
      () => handle->removeOnInsert(cb)
    })
    let upd = O.fromCallback(emit => {
      let cb = (ctx: {{self.sdk_module}}.eventCtx, prev: t, next: t) => emit((ctx, Updated({prev: prev, next: next})))
      handle->onUpdate(cb)
      () => handle->removeOnUpdate(cb)
    })
    let del = O.fromCallback(emit => {
      let cb = (ctx: {{self.sdk_module}}.eventCtx, row: t) => emit((ctx, Deleted({row: row})))
      handle->onDelete(cb)
      () => handle->removeOnDelete(cb)
    })
    O.merge([ins, upd, del])
  }
}
