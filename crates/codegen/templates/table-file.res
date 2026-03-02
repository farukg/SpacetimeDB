{{self.header}}

open {{self.root_module}}

{{self.row_type}}
// Opaque table handle — obtained from Client.db
type handle

@send external iter: handle => Iterator.t<t> = "iter"

@send external onInsert: (handle, (Sdk.eventCtx, t) => unit) => unit = "onInsert"
@send external removeOnInsert: (handle, (Sdk.eventCtx, t) => unit) => unit = "removeOnInsert"
@send external onUpdate: (handle, (Sdk.eventCtx, t, t) => unit) => unit = "onUpdate"
@send external removeOnUpdate: (handle, (Sdk.eventCtx, t, t) => unit) => unit = "removeOnUpdate"
@send external onDelete: (handle, (Sdk.eventCtx, t) => unit) => unit = "onDelete"
@send external removeOnDelete: (handle, (Sdk.eventCtx, t) => unit) => unit = "removeOnDelete"

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
%% if !self.display_section.is_empty() {
{{self.display_section}}
%% }
