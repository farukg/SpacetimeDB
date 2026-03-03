// React hook — typed query binding
@module("{{self.schema_module_path}}.res.mjs") @scope("tables") @val
external query: React.query<t> = "{{self.accessor}}"

let useRows = () => React.useTable(query)
let useRowsState = () => React.useTableState(query)
let useRowsWith = (~onInsert=?, ~onUpdate=?, ~onDelete=?) =>
  React.useTableWith(query, {?onInsert, ?onDelete, ?onUpdate})
%% if self.has_display {
let useDisplayRows = () => React.useTable(query)->Array.map(toDisplay)
%% }
%% if self.has_pk {
%% if self.pk_is_identity {
let useRow = (id: {{self.pk_type}}) =>
  React.useTable(query)->Array.find(row => Sdk.identityIsEqual(row.{{self.pk_field_camel}}, id))
%% } else {
let useRow = (id: {{self.pk_type}}) =>
  React.useTable(query)->Array.find(row => row.{{self.pk_field_camel}} == id)
%% }
%% }
