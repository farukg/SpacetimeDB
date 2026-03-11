{{self.header}}

// Server-side reducer wrappers — module functor with typed reducer calls.
//
// Usage:
//   module ServerReducers = StdbServerReducers.Make({
//     let getConnection = () => ServerConn.getConnection()
//   })
//   let result = ServerReducers.serverReducers.addPlayer({name: "Alice"})

{{self.sibling_opens}}

module type Config = {
  let getConnection: unit => Fx.call<Sdk.connection<Sdk.remoteModule>>
}

module Make = (R: Fx.CALL_RUNTIME, C: Config) => {
%% for w in &self.reducer_wrappers {

{{w}}
%% }

%% if self.has_reducers {

  type serverReducers = {
{{self.reducer_type_fields}}
  }

  let serverReducers: serverReducers = {
{{self.reducer_value_fields}}
  }
%% }
}
