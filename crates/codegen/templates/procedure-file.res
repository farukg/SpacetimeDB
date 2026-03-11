{{self.header}}
{{self.sibling_opens}}

{{self.params_record}}
type response = {{self.result_type}}
%% if self.is_result {
type wireResponse = Sdk.sdkResult<{{self.ok_type}}, {{self.err_type}}>
%% }
type rawCall<'a> = Stdb__CallSupport.foreignCall<'a>
external fromSupportCall: Stdb__CallSupport.call<'a> => Fx.call<'a> = "%identity"

let wrapCall = raw =>
  raw->Stdb__CallSupport.fromForeignCall->fromSupportCall

let procedureName = "{{self.procedure_name}}"
%% if self.has_args {
%% if self.is_result {

@send external rawCall_: (Sdk.procedures, params) => rawCall<wireResponse> = "{{self.accessor}}"
let call_ = (procedures: Sdk.procedures, args: params): Fx.call<wireResponse> =>
  procedures->rawCall_(args)->wrapCall
let callRaw = (conn: Sdk.connection<Sdk.remoteModule>, args: params): Fx.call<wireResponse> =>
  conn->Sdk.getProcedures->call_(args)
%% } else {

@send external rawCall_: (Sdk.procedures, params) => rawCall<response> = "{{self.accessor}}"
let call_ = (procedures: Sdk.procedures, args: params): Fx.call<response> =>
  procedures->rawCall_(args)->wrapCall
let call = (conn: Sdk.connection<Sdk.remoteModule>, args: params): Fx.call<response> =>
  conn->Sdk.getProcedures->call_(args)
%% }
%% } else {
%% if self.is_result {

@send external rawCall_: Sdk.procedures => rawCall<wireResponse> = "{{self.accessor}}"
let call_ = (procedures: Sdk.procedures): Fx.call<wireResponse> =>
  procedures->rawCall_->wrapCall
let callRaw = (conn: Sdk.connection<Sdk.remoteModule>): Fx.call<wireResponse> =>
  conn->Sdk.getProcedures->call_
%% } else {

@send external rawCall_: Sdk.procedures => rawCall<response> = "{{self.accessor}}"
let call_ = (procedures: Sdk.procedures): Fx.call<response> =>
  procedures->rawCall_->wrapCall
let call = (conn: Sdk.connection<Sdk.remoteModule>): Fx.call<response> =>
  conn->Sdk.getProcedures->call_
%% }
%% }
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
