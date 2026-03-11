{{self.header}}
{{self.sibling_opens}}

type rawCall<'a> = Stdb__CallSupport.foreignCall<'a>
external fromSupportCall: Stdb__CallSupport.call<'a> => Fx.call<'a> = "%identity"

let wrapCall = raw =>
  raw->Stdb__CallSupport.fromForeignCall->fromSupportCall

%% if self.has_args {
{{self.args_record}}

@send external rawCall_: (Sdk.reducers, args) => rawCall<unit> = "{{self.accessor}}"
let call_ = (reducers: Sdk.reducers, args: args): Fx.call<unit> =>
  reducers->rawCall_(args)->wrapCall
let call = (conn: Sdk.connection<Sdk.remoteModule>, args: args): Fx.call<unit> =>
  conn->Sdk.getReducers->call_(args)
%% } else {
@send external rawCall_: Sdk.reducers => rawCall<unit> = "{{self.accessor}}"
let call_ = (reducers: Sdk.reducers): Fx.call<unit> =>
  reducers->rawCall_->wrapCall
let call = (conn: Sdk.connection<Sdk.remoteModule>): Fx.call<unit> =>
  conn->Sdk.getReducers->call_
%% }
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
