// React hook — typed reducer binding
@module("../StdbSchema.res.mjs") @scope("reducers") @val
external reducerDef: StdbReact.reducerDef<{{self.params_type}}> = "{{self.camel_accessor}}"

let useCall = () => StdbReact.useReducer(reducerDef)