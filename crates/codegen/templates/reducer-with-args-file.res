{{self.header}}
{{self.sibling_opens}}

{{self.args_record}}

@send external call_: (Sdk.reducers, args) => promise<unit> = "{{self.accessor}}"
let call = (conn: Sdk.connection<Sdk.remoteModule>, args: args): promise<unit> =>
  conn->Sdk.getReducers->call_(args)
%% if !self.make_functor.is_empty() {
{{self.make_functor}}
%% }
%% if !self.react_hooks.is_empty() {

{{self.react_hooks}}
%% }
