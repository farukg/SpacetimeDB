
// ── Observer mode ─────────────────────────────────────────────────────────────
module Make = (E: Async.EFFECT_RUNTIME) => {
%% if self.has_args {
  let call = (conn: Sdk.connection<Sdk.remoteModule>, callArgs: params): E.effect<result<response, exn>> =>
    E.fromPromise(() =>
      call(conn, callArgs)
      ->Promise.then(v => Promise.resolve(Ok(v)))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% } else {
  let call = (conn: Sdk.connection<Sdk.remoteModule>): E.effect<result<response, exn>> =>
    E.fromPromise(() =>
      call(conn)
      ->Promise.then(v => Promise.resolve(Ok(v)))
      ->Promise.catch(e => Promise.resolve(Error(e)))
    )
%% }
}
