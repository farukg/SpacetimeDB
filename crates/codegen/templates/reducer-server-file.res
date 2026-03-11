{{self.header}}
{{self.sibling_opens}}
module Reducer = {{self.reducer_module}}

module Make = (R: Fx.CALL_RUNTIME) => {
%% if self.has_args {
  let call = (conn: Sdk.connection<Sdk.remoteModule>, args: Reducer.args): Fx.call<result<unit, Fx.error>> =>
    conn->Sdk.getReducers->Reducer.call_(args)->R.capture
%% } else {
  let call = (conn: Sdk.connection<Sdk.remoteModule>): Fx.call<result<unit, Fx.error>> =>
    conn->Sdk.getReducers->Reducer.call_->R.capture
%% }
}
