// React hook — typed reducer binding
@module("../StdbSchema.res.mjs") @scope("reducers") @val
external reducerDef: {{self.react_module}}.reducerDef<{{self.params_type}}> = "{{self.camel_accessor}}"

let useCall = () => {{self.react_module}}.useMutation(reducerDef)
let useCallFn = () => {{self.react_module}}.useReducer(reducerDef)