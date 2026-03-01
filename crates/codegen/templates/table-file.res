{{self.header}}

{{self.row_type}}
// Opaque table handle — obtained from StdbClient.db
type handle

@send external iter: handle => Iterator.t<t> = "iter"

@send external onInsert: (handle, (StdbSdk.eventCtx, t) => unit) => unit = "onInsert"
@send external removeOnInsert: (handle, (StdbSdk.eventCtx, t) => unit) => unit = "removeOnInsert"
@send external onUpdate: (handle, (StdbSdk.eventCtx, t, t) => unit) => unit = "onUpdate"
@send external removeOnUpdate: (handle, (StdbSdk.eventCtx, t, t) => unit) => unit = "removeOnUpdate"
@send external onDelete: (handle, (StdbSdk.eventCtx, t) => unit) => unit = "onDelete"
@send external removeOnDelete: (handle, (StdbSdk.eventCtx, t) => unit) => unit = "removeOnDelete"

%% if self.has_deleted_at {
let isAlive = (row: t) => row.deletedAt->Option.isNone

%% }
%% if !self.pk_section.is_empty() {
{{self.pk_section}}

%% }
let tableName = "{{self.table_name}}"

{{self.react_hooks}}