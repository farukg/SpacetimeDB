// React hook — typed reducer binding
@module("../{{self.schema_module}}.res.mjs") @scope("reducers") @val
external reducerDef: React.reducerDef<{{self.params_type}}> = "{{self.camel_accessor}}"

let useCall = () => React.useMutation(reducerDef)
let useCallFn = () => React.useReducer(reducerDef)
