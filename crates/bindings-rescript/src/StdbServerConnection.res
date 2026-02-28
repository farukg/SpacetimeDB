// Singleton server connection manager for SpacetimeDB.
// Manages config, lazy connection with timeout, and subscription setup.

// --- External bindings for spacetimedb/sdk ---

type remoteModule
type dbConnectionBuilder
type dbConnectionImpl
type dbConfig
type connection
type subscriptionBuilder

@new @module("spacetimedb/sdk")
external makeDbConnectionBuilder: (remoteModule, dbConfig => dbConnectionImpl) => dbConnectionBuilder =
  "DbConnectionBuilder"

@new @module("spacetimedb/sdk")
external makeDbConnectionImpl: dbConfig => dbConnectionImpl = "DbConnectionImpl"

@send
external withUri: (dbConnectionBuilder, string) => dbConnectionBuilder = "withUri"

@send
external withDatabaseName: (dbConnectionBuilder, string) => dbConnectionBuilder = "withDatabaseName"

@send
external onConnect: (dbConnectionBuilder, connection => unit) => dbConnectionBuilder = "onConnect"

@send
external onConnectError: (dbConnectionBuilder, ('ctx, JsExn.t) => unit) => dbConnectionBuilder =
  "onConnectError"

@send
external buildConnection: dbConnectionBuilder => connection = "build"

@get external isActive: connection => bool = "isActive"
@send external disconnect: connection => unit = "disconnect"
@send external subscriptionBuilder: connection => subscriptionBuilder = "subscriptionBuilder"

@send
external onApplied: (subscriptionBuilder, unit => unit) => subscriptionBuilder = "onApplied"

@send
external onSubError: (subscriptionBuilder, ('ctx, JsExn.t) => unit) => subscriptionBuilder = "onError"

@send
external subscribe: (subscriptionBuilder, array<string>) => unit = "subscribe"

@val @scope("Promise")
external race: array<promise<'a>> => promise<'a> = "race"

@val external setTimeout: (unit => unit, int) => float = "setTimeout"

// --- Config type ---

type config = {
  remoteModule: remoteModule,
  databaseName: string,
  uri: string,
  allTables?: array<string>,
  connectTimeoutMs?: int,
}

// --- Module state ---

let serverConnectionConfig: ref<option<config>> = ref(None)
let serverConnectionPromise: ref<option<promise<connection>>> = ref(None)
let serverConnection: ref<option<connection>> = ref(None)

// --- Internal helpers ---

let defaultTimeoutMs = 10000

let disconnectIfActive = () =>
  switch serverConnection.contents {
  | Some(conn) if conn->isActive => conn->disconnect
  | Some(_) | None => ()
  }

// Coerce any error value to exn for Promise rejection.
// SDK callbacks deliver Exn.t (JS Error instances) which are a subset of exn.
let toExn: 'a => exn = Obj.magic

let withTimeout = (connectionPromise, timeoutMs) =>
  switch timeoutMs {
  | None | Some(0) => connectionPromise
  | Some(ms) if ms < 0 => connectionPromise
  | Some(ms) =>
    let timeoutPromise = Promise.make((_resolve, reject) => {
      setTimeout(() => {
        reject(
          JsExn.anyToExnInternal(
            JsError.make(`Timed out waiting for SpacetimeDB connection after ${ms->Int.toString}ms`),
          ),
        )
      }, ms)->ignore
    })
    race([connectionPromise, timeoutPromise])
  }

let connectServerConnection = config => {
  let {remoteModule, databaseName, uri, ?allTables, ?connectTimeoutMs} = config
  let timeoutMs = connectTimeoutMs->Option.getOr(defaultTimeoutMs)

  let builder =
    makeDbConnectionBuilder(remoteModule, dbConfig => makeDbConnectionImpl(dbConfig))
    ->withUri(uri)
    ->withDatabaseName(databaseName)

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
    ->onConnect(conn => {
      try {
        switch allTables {
        | Some(tables) if tables->Array.length > 0 =>
          let queries = tables->Array.map(tableName => `SELECT * FROM ${tableName}`)
          conn
          ->subscriptionBuilder
          ->onApplied(() => resolveOnce(conn))
          ->onSubError((_ctx, error) => rejectOnce(error->Obj.magic))
          ->subscribe(queries)
        | Some(_) | None => resolveOnce(conn)
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
      serverConnection := Some(conn)
    } catch {
    | JsExn(jsExn) => rejectOnce(jsExn->Obj.magic)
    | exn => rejectOnce(exn->Obj.magic)
    }
  })

  connectPromise->withTimeout(Some(timeoutMs))
}

// --- Public API ---

let configureServerConnection = config => {
  disconnectIfActive()
  serverConnectionConfig := Some(config)
  serverConnection := None
  serverConnectionPromise := None
}

let getConnection = () =>
  switch serverConnectionConfig.contents {
  | None =>
    JsError.throwWithMessage(
      "Server connection is not configured. Call configureServerConnection(config) first.",
    )
  | Some(config) =>
    switch serverConnectionPromise.contents {
    | Some(existingPromise) => existingPromise
    | None =>
      let p =
        connectServerConnection(config)->Promise.catch(error => {
          serverConnectionPromise := None
          throw(error)
        })
      serverConnectionPromise := Some(p)
      p
    }
  }

let resetServerConnectionForTests = () => {
  disconnectIfActive()
  serverConnection := None
  serverConnectionConfig := None
  serverConnectionPromise := None
}
