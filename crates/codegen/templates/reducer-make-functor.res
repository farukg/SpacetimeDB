
// ── Observer mode ─────────────────────────────────────────────────────────────
module Make = (E: Stdb__Async.EFFECT_RUNTIME) => {
%% if self.has_args {
  let call = (conn: {{self.sdk_module}}.connection, callArgs: args): E.effect<result<unit, exn>> =>
    E.fromPromise(() =>
      conn->StdbClient.reducers->{{self.accessor}}(callArgs)
      ->Promise.then(_ => Promise.resolve(Ok()))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% } else {
  let call = (conn: {{self.sdk_module}}.connection): E.effect<result<unit, exn>> =>
    E.fromPromise(() =>
      conn->StdbClient.reducers->{{self.accessor}}
      ->Promise.then(_ => Promise.resolve(Ok()))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% }
}
