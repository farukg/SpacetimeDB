{{self.header}}

{{self.sibling_opens}}

{{self.row_type}}
// Opaque table handle — obtained from Client.db
type handle

@send external iter: handle => Iterator.t<t> = "iter"

@send external onInsert: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit = "onInsert"
@send external removeOnInsert: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit = "removeOnInsert"
@send external onUpdate: (handle, @uncurry (Sdk.eventCtx, t, t) => unit) => unit = "onUpdate"
@send external removeOnUpdate: (handle, @uncurry (Sdk.eventCtx, t, t) => unit) => unit = "removeOnUpdate"
@send external onDelete: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit = "onDelete"
@send external removeOnDelete: (handle, @uncurry (Sdk.eventCtx, t) => unit) => unit = "removeOnDelete"

%% if !self.pk_section.is_empty() {
{{self.pk_section}}

%% }
let tableName = "{{self.table_name}}"
%% if self.uses_functor {

// ── Shared boilerplate via functor ────────────────────────────────────────────
include TableFunctor.Make({
  type t = t
  type handle = handle
  let iter = iter
  let onInsert = onInsert
  let removeOnInsert = removeOnInsert
  let onUpdate = onUpdate
  let removeOnUpdate = removeOnUpdate
  let onDelete = onDelete
  let removeOnDelete = removeOnDelete
})
%% } else {
{{self.event_section}}
%% if !self.observer_section.is_empty() {
{{self.observer_section}}
%% }
%% }
%% if !self.display_section.is_empty() {
{{self.display_section}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
