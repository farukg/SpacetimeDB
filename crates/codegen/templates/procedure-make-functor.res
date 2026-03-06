
// ── Observer mode ─────────────────────────────────────────────────────────────
module Make = (E: Async.EFFECT_RUNTIME) => {
%% if self.has_args {
  let call = (conn: Sdk.connection, callArgs: params): E.effect<result<response, exn>> =>
    E.fromPromise(() =>
      conn->Sdk.getProcedures->call_(callArgs)
      ->Promise.then(v => Promise.resolve(Ok(v)))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% } else {
  let call = (conn: Sdk.connection): E.effect<result<response, exn>> =>
    E.fromPromise(() =>
      conn->Sdk.getProcedures->call_
      ->Promise.then(v => Promise.resolve(Ok(v)))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% }
}
