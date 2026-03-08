{{self.header}}
{{self.sibling_opens}}

{{self.params_record}}
type response = {{self.result_type}}

let procedureName = "{{self.procedure_name}}"
%% if self.has_args {
%% if self.is_result {

@send external call_: (Sdk.procedures, params) => promise<Sdk.sdkResult<{{self.ok_type}}, {{self.err_type}}>> = "{{self.accessor}}"
let call = (conn: Sdk.connection, args: params): promise<response> =>
  conn->Sdk.getProcedures->call_(args)->Promise.then(raw => Promise.resolve(Sdk.fromSdkResult(raw)))
%% } else {

@send external call_: (Sdk.procedures, params) => promise<response> = "{{self.accessor}}"
let call = (conn: Sdk.connection, args: params): promise<response> =>
  conn->Sdk.getProcedures->call_(args)
%% }
%% } else {
%% if self.is_result {

@send external call_: Sdk.procedures => promise<Sdk.sdkResult<{{self.ok_type}}, {{self.err_type}}>> = "{{self.accessor}}"
let call = (conn: Sdk.connection): promise<response> =>
  conn->Sdk.getProcedures->call_->Promise.then(raw => Promise.resolve(Sdk.fromSdkResult(raw)))
%% } else {

@send external call_: Sdk.procedures => promise<response> = "{{self.accessor}}"
let call = (conn: Sdk.connection): promise<response> =>
  conn->Sdk.getProcedures->call_
%% }
%% }
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
