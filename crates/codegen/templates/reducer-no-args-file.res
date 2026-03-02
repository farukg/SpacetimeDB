{{self.header}}

@send external {{self.accessor}}: {{self.sdk_module}}.reducers => promise<unit> = "{{self.accessor}}"

let call = (conn: {{self.sdk_module}}.connection) =>
  conn->StdbClient.reducers->{{self.accessor}}
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
