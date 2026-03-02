{{self.header}}
%% if self.has_args {

open {{self.reducer_module}}

let call = async (conn: {{self.sdk_module}}.connection, args: args) => {
  try {
    await conn->StdbClient.reducers->{{self.accessor}}(args)
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% } else {

let call = async (conn: {{self.sdk_module}}.connection) => {
  try {
    await conn->StdbClient.reducers->{{self.accessor}}
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% }