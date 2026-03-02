{{self.header}}
{{self.sibling_opens}}

@send external {{self.accessor}}: Sdk.reducers => promise<unit> = "{{self.accessor}}"

let call = (conn: Sdk.connection) =>
  conn->Client.reducers->{{self.accessor}}
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
