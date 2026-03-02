{{self.header}}

{{self.row_type}}
// Opaque table handle — obtained from StdbClient.db
type handle

@send external iter: handle => Iterator.t<t> = "iter"

@send external onInsert: (handle, ({{self.sdk_module}}.eventCtx, t) => unit) => unit = "onInsert"
@send external removeOnInsert: (handle, ({{self.sdk_module}}.eventCtx, t) => unit) => unit = "removeOnInsert"
@send external onUpdate: (handle, ({{self.sdk_module}}.eventCtx, t, t) => unit) => unit = "onUpdate"
@send external removeOnUpdate: (handle, ({{self.sdk_module}}.eventCtx, t, t) => unit) => unit = "removeOnUpdate"
@send external onDelete: (handle, ({{self.sdk_module}}.eventCtx, t) => unit) => unit = "onDelete"
@send external removeOnDelete: (handle, ({{self.sdk_module}}.eventCtx, t) => unit) => unit = "removeOnDelete"

%% if !self.pk_section.is_empty() {
{{self.pk_section}}

%% }
let tableName = "{{self.table_name}}"
{{self.event_section}}
%% if !self.observer_section.is_empty() {
{{self.observer_section}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
