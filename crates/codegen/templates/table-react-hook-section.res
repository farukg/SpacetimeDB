// React hook — typed query binding
@module("../{{self.schema_module}}.res.mjs") @scope("tables") @val
external query: {{self.react_module}}.query<t> = "{{self.accessor}}"

let useRows = () => {{self.react_module}}.useTable(query)
let useRowsState = () => {{self.react_module}}.useTableState(query)
let useRowsWith = (~onInsert=?, ~onUpdate=?, ~onDelete=?) =>
  {{self.react_module}}.useTableWith(query, {?onInsert, ?onDelete, ?onUpdate})
%% if self.has_display {
let useDisplayRows = () => {{self.react_module}}.useTable(query)->Array.map(toDisplay)
%% }
%% if self.has_pk {
let useRow = (id: {{self.pk_type}}) =>
  {{self.react_module}}.useTable(query)->Array.find(row => row.{{self.pk_field_camel}} == id)
%% }