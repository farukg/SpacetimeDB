{{self.header}}
{{self.sibling_opens}}

{{self.params_record}}
type response = {{self.result_type}}

let procedureName = "{{self.procedure_name}}"
%% if self.has_args {
@send external {{self.accessor}}: (Sdk.procedures, params) => promise<response> = "{{self.accessor}}"

let call = (conn: Sdk.connection, args: params) =>
  conn->Client.procedures->{{self.accessor}}(args)
%% } else {
@send external {{self.accessor}}: Sdk.procedures => promise<response> = "{{self.accessor}}"

let call = (conn: Sdk.connection) =>
  conn->Client.procedures->{{self.accessor}}
%% }
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
