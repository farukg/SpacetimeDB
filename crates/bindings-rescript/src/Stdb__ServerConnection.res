// StdbServerConnection — module functor for server-side SpacetimeDB connections.
//
// Usage:
//   module Config = {
//     let remoteModule = StdbSchema.remoteModule
//     let allTableNames = StdbSchema.allTableNames
//     let databaseName = Env.spacetimedbDatabase
//     let uri = Env.spacetimedbUri
//     let connectTimeoutMs = 10000
//   }
//   module ServerConn = StdbServerConnection.Make(Config)
//   let conn = await ServerConn.getConnection()

@@warning("-44")
open Stdb__Sdk

module type Config = {
  let remoteModule: remoteModule
  let allTableNames: array<string>
  let databaseName: string
  let uri: string
  let connectTimeoutMs: int
}

module type S = {
  let getConnection: unit => promise<connection>
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

module Make = (C: Config): S => {
  let connectionRef: ref<option<connection>> = ref(None)
  let promiseRef: ref<option<promise<connection>>> = ref(None)

  let disconnectIfActive = () =>
    switch connectionRef.contents {
    | Some(conn) if conn->isActive => conn->disconnect
    | Some(_) | None => ()
    }

  let connect = () => {
    let builder =
      makeDbConnectionBuilder(C.remoteModule, dbConfig => makeDbConnectionImpl(dbConfig))
      ->withUri(C.uri)
      ->withDatabaseName(C.databaseName)

    let connectPromise = Promise.make((resolve, reject) => {
      let settled = ref(false)

      let resolveOnce = conn => {
        switch settled.contents {
        | true => ()
        | false =>
          settled := true
          resolve(conn)
        }
      }

      let rejectOnce = error => {
        switch settled.contents {
        | true => ()
        | false =>
          settled := true
          disconnectIfActive()
          reject(toExn(error))
        }
      }

      builder
      ->onConnect((conn, _identity, _token) => {
        try {
          switch C.allTableNames {
          | tables if tables->Array.length > 0 =>
            let queries = tables->Array.map(tableName => `SELECT * FROM ${tableName}`)
            conn
            ->subscriptionBuilder
            ->onApplied(() => resolveOnce(conn))
            ->onSubError((_ctx, error) => rejectOnce(error->Obj.magic))
            ->subscribe(queries)
          | _ => resolveOnce(conn)
          }
        } catch {
        | JsExn(jsExn) => rejectOnce(jsExn->Obj.magic)
        | exn => rejectOnce(exn->Obj.magic)
        }
      })
      ->onConnectError((_ctx, error) => rejectOnce(error->Obj.magic))
      ->ignore

      try {
        let conn = builder->buildConnection
        connectionRef := Some(conn)
      } catch {
      | JsExn(jsExn) => rejectOnce(jsExn->Obj.magic)
      | exn => rejectOnce(exn->Obj.magic)
      }
    })

    connectPromise->withTimeout(C.connectTimeoutMs)
  }

  let getConnection = () =>
    switch promiseRef.contents {
    | Some(existingPromise) => existingPromise
    | None =>
      let p =
        connect()->Promise.catch(error => {
          promiseRef := None
          throw(error)
        })
      promiseRef := Some(p)
      p
    }

  let resetForTests = () => {
    disconnectIfActive()
    connectionRef := None
    promiseRef := None
  }
}
