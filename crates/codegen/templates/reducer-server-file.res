{{self.header}}
{{self.sibling_opens}}
module Reducer = {{self.reducer_module}}
%% if self.has_args {

let call = async (conn: Sdk.connection, args: Reducer.args) => {
  try {
    await conn->Client.reducers->Reducer.{{self.accessor}}(args)
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% } else {

let call = async (conn: Sdk.connection) => {
  try {
    await conn->Client.reducers->Reducer.{{self.accessor}}
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% }
