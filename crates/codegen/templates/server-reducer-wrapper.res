%% if self.has_args {
  let {{self.name_camel}} = (args: {{self.module}}.args): Fx.call<result<unit, Fx.error>> =>
%% } else {
  let {{self.name_camel}} = (): Fx.call<result<unit, Fx.error>> =>
%% }
    C.getConnection()->R.flatMap(conn =>
%% if self.has_args {
      conn->Sdk.getReducers->{{self.module}}.call_(args)->R.capture
%% } else {
      conn->Sdk.getReducers->{{self.module}}.call_->R.capture
%% }
    )
