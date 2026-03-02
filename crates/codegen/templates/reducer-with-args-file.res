{{self.header}}
open {{self.root_module}}

{{self.args_record}}
@send external {{self.accessor}}: (Sdk.reducers, args) => promise<unit> = "{{self.accessor}}"

let call = (conn: Sdk.connection, args: args) =>
  conn->Client.reducers->{{self.accessor}}(args)
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
