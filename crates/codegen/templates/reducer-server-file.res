{{self.header}}
{{self.sibling_opens}}
module Self = {{self.reducer_module}}
%% if self.has_args {

let call = async (conn: Sdk.connection, args: Self.args) => {
  try {
    await conn->Client.reducers->Self.{{self.accessor}}(args)
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% } else {

let call = async (conn: Sdk.connection) => {
  try {
    await conn->Client.reducers->Self.{{self.accessor}}
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% }
