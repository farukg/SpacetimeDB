// React hook — typed query binding
@module("../StdbSchema.res.mjs") @scope("tables") @val
external query: StdbReact.query<t> = "{{self.accessor}}"

let useRows = () => StdbReact.useTable(query)
let useRowsWith = (cbs) => StdbReact.useTableWith(query, cbs)