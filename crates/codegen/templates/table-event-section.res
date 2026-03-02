
// ── Typed event union ─────────────────────────────────────────────────────────
type event =
  | Inserted({row: t})
  | Updated({prev: t, next: t})
  | Deleted({row: t})

let subscribe = (handle: handle, handler: event => unit): (unit => unit) => {
  let insertCb = (_ctx: Sdk.eventCtx, row: t) => handler(Inserted({row: row}))
  let updateCb = (_ctx: Sdk.eventCtx, prev: t, next: t) => handler(Updated({prev: prev, next: next}))
  let deleteCb = (_ctx: Sdk.eventCtx, row: t) => handler(Deleted({row: row}))
  handle->onInsert(insertCb)
  handle->onUpdate(updateCb)
  handle->onDelete(deleteCb)
  () => {
    handle->removeOnInsert(insertCb)
    handle->removeOnUpdate(updateCb)
    handle->removeOnDelete(deleteCb)
  }
}
