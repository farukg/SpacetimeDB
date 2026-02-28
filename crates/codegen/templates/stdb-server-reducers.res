{{self.header}}

// Server-side reducer wrappers — module functor with typed, async reducers.
//
// Usage:
//   module ServerReducers = StdbServerReducers.Make({
//     let getConnection = () => ServerConn.getConnection()
//   })
//   let result = await ServerReducers.serverReducers.addPlayer({name: "Alice"})

open StdbSdk

module type Config = {
  let getConnection: unit => promise<connection>
}

module Make = (C: Config) => {
  @val external setTimeout: (unit => unit, int) => float = "setTimeout"

  @val @scope(("process", "env"))
  external syncDelayMsEnv: option<string> = "STDB_TYPED_REDUCER_SYNC_DELAY_MS"

  let syncDelayMs = switch syncDelayMsEnv {
  | Some(s) => Int.fromString(s)->Option.getOr(300)
  | None => 300
  }

  let sleep = (ms) => {
    Promise.make((resolve, _reject) => {
      let _ = setTimeout(() => resolve(), ms)
    })
  }
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
