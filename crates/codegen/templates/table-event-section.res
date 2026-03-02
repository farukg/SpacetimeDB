
// ── Typed event union ─────────────────────────────────────────────────────────
type event =
  | Inserted({row: t})
  | Updated({prev: t, next: t})
  | Deleted({row: t})

let subscribe = (handle: handle, handler: event => unit): (unit => unit) => {
  let insertCb = (_ctx: {{self.sdk_module}}.eventCtx, row: t) => handler(Inserted({row: row}))
  let updateCb = (_ctx: {{self.sdk_module}}.eventCtx, prev: t, next: t) => handler(Updated({prev: prev, next: next}))
  let deleteCb = (_ctx: {{self.sdk_module}}.eventCtx, row: t) => handler(Deleted({row: row}))
  handle->onInsert(insertCb)
  handle->onUpdate(updateCb)
  handle->onDelete(deleteCb)
  () => {
    handle->removeOnInsert(insertCb)
    handle->removeOnUpdate(updateCb)
    handle->removeOnDelete(deleteCb)
  }
}
