{{self.header}}

{{self.args_record}}
@send external {{self.accessor}}: ({{self.sdk_module}}.reducers, args) => promise<unit> = "{{self.accessor}}"

let call = (conn: {{self.sdk_module}}.connection, args: args) =>
  conn->StdbClient.reducers->{{self.accessor}}(args)
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }