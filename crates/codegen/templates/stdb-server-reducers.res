{{self.header}}

// Server-side reducer wrappers — module functor with typed, async reducers.
//
// Usage:
//   module ServerReducers = StdbServerReducers.Make({
//     let getConnection = () => ServerConn.getConnection()
//   })
//   let result = await ServerReducers.serverReducers.addPlayer({name: "Alice"})

{{self.sibling_opens}}

module type Config = {
  let getConnection: unit => promise<Sdk.connection>
}

module Make = (C: Config) => {
%% for w in &self.reducer_wrappers {

{{w}}
%% }

%% if self.has_reducers {

  type serverReducers = {
%% for f in &self.reducer_type_fields {
{{f}}
%% }
  }

  let serverReducers: serverReducers = {
%% for f in &self.reducer_value_fields {
{{f}}
%% }
  }
%% }
}
