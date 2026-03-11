// Stdb__ServerConnection — shared server-side connection slotting.
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
//   let connCall = ServerConn.getConnection()

@@warning("-44")
open Stdb__SdkBindings

module Support = Stdb__CallSupport

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
  let getConnection: unit => Support.call<dbConnectionImpl<rm>>
  let getConnectionWithToken: option<string> => Support.call<dbConnectionImpl<rm>>
  let resetForTests: unit => unit
}

type listener<'a> = {
  id: int,
  onValue: 'a => unit,
  onError: Support.issue => unit,
}

type slot<'a> = {
  mutable nextId: int,
  mutable current: option<'a>,
  mutable listeners: array<listener<'a>>,
  mutable cancelCurrent: option<Support.cleanup>,
}

let makeSlot = (): slot<'a> => {
  nextId: 0,
  current: None,
  listeners: [],
  cancelCurrent: None,
}

let stopSlot = (slot: slot<dbConnectionImpl<'a>>) => {
  switch slot.cancelCurrent {
  | Some(cancel) => cancel()
  | None => ()
  }
  switch slot.current {
  | Some(conn) => conn->disconnect
  | None => ()
  }
  slot.nextId = 0
  slot.current = None
  slot.listeners = []
  slot.cancelCurrent = None
}

let deliverValue = (slot: slot<'a>, value) => {
  slot.current = Some(value)
  slot.cancelCurrent = None
  let listeners = slot.listeners
  slot.listeners = []
  listeners->Array.forEach(listener => listener.onValue(value))
}

let deliverIssue = (slot: slot<'a>, issue) => {
  slot.cancelCurrent = None
  let listeners = slot.listeners
  slot.listeners = []
  listeners->Array.forEach(listener => listener.onError(issue))
}

let removeListener = (slot: slot<'a>, listenerId: int) => {
  slot.listeners = slot.listeners->Array.filter(listener => listener.id !== listenerId)
  switch (slot.listeners->Array.length, slot.cancelCurrent) {
  | (0, Some(cancel)) =>
    slot.cancelCurrent = None
    cancel()
  | _ => ()
  }
}

let sharedCall = (slot: slot<'a>, start: unit => Support.call<'a>): Support.call<'a> =>
  (~onValue, ~onError) => {
    switch slot.current {
    | Some(value) =>
      onValue(value)
      () => ()
    | None =>
      let listenerId = slot.nextId
      slot.nextId = slot.nextId + 1
      slot.listeners = Array.concat(slot.listeners, [{id: listenerId, onValue, onError}])

      switch slot.cancelCurrent {
      | Some(_) => ()
      | None =>
        let cancel =
          start()->Support.observe(
            ~onValue=value => deliverValue(slot, value),
            ~onError=issue => deliverIssue(slot, issue),
          )
        slot.cancelCurrent = Some(cancel)
      }

      () => removeListener(slot, listenerId)
    }
  }

let connectWithConfig = (
  ~remoteModule: Stdb__Sdk.remoteModule,
  ~uri,
  ~databaseName,
  ~allTableNames,
  ~token,
  ~timeoutMs,
): Support.call<dbConnectionImpl<'a>> =>
  (~onValue, ~onError) => {
    let settled = ref(false)
    let currentConn: ref<option<dbConnectionImpl<'a>>> = ref(None)

    let finishWithValue = conn =>
      switch settled.contents {
      | true => ()
      | false =>
        settled := true
        onValue(conn)
      }

    let finishWithIssue = issue =>
      switch settled.contents {
      | true => ()
      | false =>
        settled := true
        onError(issue)
      }

    switch timeoutMs {
    | ms if ms > 0 =>
      setTimeout(
        () => finishWithIssue(Support.TimedOut({milliseconds: ms})),
        ms,
      )->ignore
    | _ => ()
    }

    let builder =
      Stdb__Normalize.makeNormalizedBuilder(remoteModule, dbConfig => makeDbConnectionImpl(dbConfig))
      ->withUri(uri)
      ->withDatabaseName(databaseName)
      ->withToken(token)
      ->onConnect((conn, _identity, _authToken) => {
        currentConn := Some(conn)
        switch allTableNames {
        | tables if tables->Array.length > 0 =>
          let queries = tables->Array.map(tableName => `SELECT * FROM ${tableName}`)
          conn
          ->subscriptionBuilder
          ->onApplied(_ctx => finishWithValue(conn))
          ->onSubError((_ctx, issue) => finishWithIssue(issue->Support.fromCallbackIssue))
          ->subscribe(queries)
          ->ignore
        | _ => finishWithValue(conn)
        }
      })
      ->onConnectError((_ctx, issue) => finishWithIssue(issue->Support.fromCallbackIssue))

    builder->build->ignore

    () => {
      settled := true
      switch currentConn.contents {
      | Some(conn) => conn->disconnect
      | None => ()
      }
    }
  }

module Make = (C: Config): (S with type rm = C.rm) => {
  type rm = C.rm

  let defaultSlot: slot<dbConnectionImpl<rm>> = makeSlot()
  let tokenPool: ref<Dict.t<slot<dbConnectionImpl<rm>>>> = ref(Dict.make())

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
    sharedCall(defaultSlot, () => connectOne(~token=None))

  let getConnectionWithToken = token => {
    let key = switch token {
    | Some(value) => value
    | None => "__anonymous__"
    }

    let slot =
      switch tokenPool.contents->Dict.get(key) {
      | Some(existing) => existing
      | None =>
        let created: slot<dbConnectionImpl<rm>> = makeSlot()
        tokenPool.contents->Dict.set(key, created)
        created
      }

    sharedCall(slot, () => connectOne(~token))
  }

  let resetForTests = () => {
    stopSlot(defaultSlot)
    tokenPool.contents->Dict.valuesToArray->Array.forEach(stopSlot)
    tokenPool := Dict.make()
  }
}
