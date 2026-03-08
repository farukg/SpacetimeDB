// Stdb__ServerConnection — module functor for server-side SpacetimeDB connections.
//
// Usage:
//   module Config = {
//     let remoteModule = Stdb__Schema.remoteModule
//     let allTableNames = Stdb__Schema.allTableNames
//     let databaseName = Env.spacetimedbDatabase
//     let uri = Env.spacetimedbUri
//     let connectTimeoutMs = 10000
//   }
//   module ServerConn = Stdb__ServerConnection.Make(Config)
//   let conn = await ServerConn.getConnection()

@@warning("-44")
open Stdb__SdkBindings

module type Config = {
  type rm
  let remoteModule: Stdb__Sdk.remoteModule
  let allTableNames: array<string>
  let databaseName: string
  let uri: string
  let connectTimeoutMs: int
}

module type S = {
  type rm
  let getConnection: unit => promise<dbConnectionImpl<rm>>
  let getConnectionWithToken: option<string> => promise<dbConnectionImpl<rm>>
  let resetForTests: unit => unit
}

// Coerce any error value to exn for Promise rejection.
// SDK callbacks deliver Exn.t (JS Error instances) which are a subset of exn.
let toExn: 'a => exn = Obj.magic

let withTimeout = (connectionPromise, timeoutMs) =>
  switch timeoutMs {
  | 0 => connectionPromise
  | ms if ms < 0 => connectionPromise
  | ms =>
    let timeoutPromise = Promise.make((_resolve, reject) => {
      setTimeout(() => {
        reject(
          JsExn.anyToExnInternal(
            JsError.make(`Timed out waiting for SpacetimeDB connection after ${ms->Int.toString}ms`),
          ),
        )
      }, ms)->ignore
    })
    promiseRace([connectionPromise, timeoutPromise])
  }

// Shared connect logic — parametric over optional token.
let connectWithConfig = (
  ~remoteModule: Stdb__Sdk.remoteModule,
  ~uri,
  ~databaseName,
  ~allTableNames,
  ~token,
  ~timeoutMs,
) => {
  let connectPromise = Promise.make((resolve, reject) => {
    let settled = ref(false)

    let resolveOnce = conn =>
      switch settled.contents {
      | true => ()
      | false =>
        settled := true
        resolve(conn)
      }

    let rejectOnce = error =>
      switch settled.contents {
      | true => ()
      | false =>
        settled := true
        reject(toExn(error))
      }

    let builder =
      Stdb__Normalize.makeNormalizedBuilder(remoteModule, dbConfig => makeDbConnectionImpl(dbConfig))
      ->withUri(uri)
      ->withDatabaseName(databaseName)
      ->withToken(token)
      ->onConnect((conn, _identity, _authToken) => {
        try {
          switch allTableNames {
          | tables if tables->Array.length > 0 =>
            let queries = tables->Array.map(tableName => `SELECT * FROM ${tableName}`)
            conn
            ->subscriptionBuilder
            ->onApplied(() => resolveOnce(conn))
            ->onSubError((_ctx, error) => rejectOnce(error->Obj.magic))
            ->subscribe(queries)
            ->ignore
          | _ => resolveOnce(conn)
          }
        } catch {
        | JsExn(jsExn) => rejectOnce(jsExn->Obj.magic)
        | exn => rejectOnce(exn->Obj.magic)
        }
      })
      ->onConnectError((_ctx, error) => rejectOnce(error->Obj.magic))

    try {
      builder->build->ignore
    } catch {
    | JsExn(jsExn) => rejectOnce(jsExn->Obj.magic)
    | exn => rejectOnce(exn->Obj.magic)
    }
  })

  connectPromise->withTimeout(timeoutMs)
}

module Make = (C: Config): (S with type rm = C.rm) => {
  type rm = C.rm

  // Default (tokenless) singleton connection
  let promiseRef: ref<option<promise<dbConnectionImpl<rm>>>> = ref(None)

  // Token-keyed connection pool
  let tokenPool: Dict.t<promise<dbConnectionImpl<rm>>> = Dict.make()

  let connectOne = (~token) =>
    connectWithConfig(
      ~remoteModule=C.remoteModule,
      ~uri=C.uri,
      ~databaseName=C.databaseName,
      ~allTableNames=C.allTableNames,
      ~token,
      ~timeoutMs=C.connectTimeoutMs,
    )

  let getConnection = () =>
    switch promiseRef.contents {
    | Some(existingPromise) => existingPromise
    | None =>
      let p =
        connectOne(~token=None)->Promise.catch(error => {
          promiseRef := None
          throw(error)
        })
      promiseRef := Some(p)
      p
    }

  let getConnectionWithToken = token => {
    let key = switch token {
    | Some(t) => t
    | None => "__anonymous__"
    }
    switch tokenPool->Dict.get(key) {
    | Some(existing) => existing
    | None =>
      let p =
        connectOne(~token)->Promise.catch(error => {
          tokenPool->Dict.delete(key)
          throw(error)
        })
      tokenPool->Dict.set(key, p)
      p
    }
  }

  let resetForTests = () => {
    promiseRef := None
  }
}
