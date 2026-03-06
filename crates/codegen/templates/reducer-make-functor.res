
// ── Observer mode ─────────────────────────────────────────────────────────────
module Make = (E: Async.EFFECT_RUNTIME) => {
%% if self.has_args {
  let call = (conn: Sdk.connection, callArgs: args): E.effect<result<unit, exn>> =>
    E.fromPromise(() =>
      conn->Sdk.getReducers->call_(callArgs)
      ->Promise.then(_ => Promise.resolve(Ok()))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% } else {
  let call = (conn: Sdk.connection): E.effect<result<unit, exn>> =>
    E.fromPromise(() =>
      conn->Sdk.getReducers->call_
      ->Promise.then(_ => Promise.resolve(Ok()))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% }
}
