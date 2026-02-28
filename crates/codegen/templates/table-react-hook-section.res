// React hook — typed query binding
@module("../StdbSchema.res.mjs") @val
external query: StdbReact.query<t> = "tables.{{self.accessor}}"

let useRows = () => StdbReact.useTable(query)
let useRowsWith = (cbs) => StdbReact.useTableWith(query, cbs)