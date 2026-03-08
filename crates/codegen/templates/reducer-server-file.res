{{self.header}}
{{self.sibling_opens}}
module Reducer = {{self.reducer_module}}
%% if self.has_args {

let call = async (conn: Sdk.connection<Sdk.remoteModule>, args: Reducer.args) => {
  try {
    await conn->Sdk.getReducers->Reducer.call_(args)
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% } else {

let call = async (conn: Sdk.connection<Sdk.remoteModule>) => {
  try {
    await conn->Sdk.getReducers->Reducer.call_
    Ok()
  } catch {
  | exn => Error(exn)
  }
}
%% }
