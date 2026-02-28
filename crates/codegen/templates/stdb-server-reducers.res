{{self.header}}

// Server-side reducer wrappers — typed, async, with connection management.
// Each function resolves the server connection, calls the reducer, and
// optionally waits for sync delay.

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
