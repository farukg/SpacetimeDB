{{self.header}}
{{self.sibling_opens}}

{{self.params_record}}
type response = {{self.result_type}}

let procedureName = "{{self.procedure_name}}"
%% if self.has_args {

@send external call_: (Sdk.procedures, params) => promise<response> = "{{self.accessor}}"
let call = (conn: Sdk.connection, args: params): promise<response> =>
  conn->Sdk.getProcedures->call_(args)
%% } else {

@send external call_: Sdk.procedures => promise<response> = "{{self.accessor}}"
let call = (conn: Sdk.connection): promise<response> =>
  conn->Sdk.getProcedures->call_
%% }
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
