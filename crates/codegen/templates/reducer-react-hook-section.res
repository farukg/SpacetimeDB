// React hook — typed reducer binding
@module("../StdbSchema.res.mjs") @val
external reducerDef: StdbReact.reducerDef<{{self.params_type}}> = "reducers.{{self.camel_accessor}}"

let useCall = () => StdbReact.useReducer(reducerDef)