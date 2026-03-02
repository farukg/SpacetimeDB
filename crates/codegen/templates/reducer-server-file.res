{{self.header}}
open {{self.root_module}}
%% if self.has_args {

open {{self.reducer_module}}

let call = async (conn: Sdk.connection, args: args) => {
  try {
    await conn->Client.reducers->{{self.accessor}}(args)
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% } else {

let call = async (conn: Sdk.connection) => {
  try {
    await conn->Client.reducers->{{self.accessor}}
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% }