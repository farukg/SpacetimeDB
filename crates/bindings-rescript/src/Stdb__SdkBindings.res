// Stdb__SdkBindings.res — Pure external bindings to SpacetimeDB TypeScript SDK
//
// §MOD.5b: Every external here is an unverified trust assertion.
// Audited against crates/bindings-typescript/dist/*.d.ts on 2026-03-08.
//
// This file is the SINGLE trust boundary for SDK interop. All SDK
// class instances are opaque types. All method calls use @send/@get.
// No %raw, no Obj.magic, no %identity.

// ═══════════════════════════════════════════════════════════════════════
// Opaque SDK class instances
// ═══════════════════════════════════════════════════════════════════════

@genType.import(("spacetimedb", "Identity"))
type identity // SDK Identity class — wraps bigint (U256)
@genType.import(("spacetimedb", "ConnectionId"))
type connectionId // SDK ConnectionId class — wraps bigint (U128)
@genType.import(("spacetimedb", "Timestamp"))
type timestamp // SDK Timestamp class — wraps bigint (I64 micros)
@genType.import(("spacetimedb", "TimeDuration"))
type timeDuration // SDK TimeDuration class — wraps bigint (I64 micros)
@genType.import(("spacetimedb", "Uuid"))
type uuid // SDK Uuid class — wraps bigint (U128)

// ═══════════════════════════════════════════════════════════════════════
// Identity
// ═══════════════════════════════════════════════════════════════════════
// import { Identity, ConnectionId, Timestamp, TimeDuration, Uuid, ScheduleAt } from 'spacetimedb';
// import { DbConnectionBuilder, DbConnectionImpl, SubscriptionBuilderImpl, SubscriptionHandleImpl } from 'spacetimedb/sdk'
@genType.import(("spacetimedb", "Identity"))
module Identity = {
  @new @module("spacetimedb")
  external make: string => identity = "Identity"

  @module("spacetimedb") @scope("Identity")
  external fromString: string => identity = "fromString"

  @module("spacetimedb") @scope("Identity")
  external zero: unit => identity = "zero"


  @send external toHexString: identity => string = "toHexString"
  @send external isEqual: (identity, identity) => bool = "isEqual"
  @send external equals: (identity, identity) => bool = "equals"
  @send external toString: identity => string = "toString"
  @send external toUint8Array: identity => Js.TypedArray2.Uint8Array.t = "toUint8Array"
}

// ═══════════════════════════════════════════════════════════════════════
// Timestamp
// ═══════════════════════════════════════════════════════════════════════

@genType
module Timestamp = {
  @new @module("spacetimedb")
  external fromMicros: bigint => timestamp = "Timestamp"

  @module("spacetimedb") @scope("Timestamp")
  external now: unit => timestamp = "now"

  @module("spacetimedb") @scope("Timestamp")
  external fromDate: Date.t => timestamp = "fromDate"

  @module("spacetimedb") @scope("Timestamp")
  external unixEpoch: timestamp = "UNIX_EPOCH"

  @send external toMillis: timestamp => bigint = "toMillis"
  @send external toDate: timestamp => Date.t = "toDate"
  @send external toISOString: timestamp => string = "toISOString"
  @send external since: (timestamp, timestamp) => timeDuration = "since"
  @get external microsSinceUnixEpoch: timestamp => bigint = "microsSinceUnixEpoch"

  let toFloatMs = ts => ts->toMillis->BigInt.toFloat
}

// ═══════════════════════════════════════════════════════════════════════
// ConnectionId
// ═══════════════════════════════════════════════════════════════════════

@genType
module ConnectionId = {
  @new @module("spacetimedb")
  external make: bigint => connectionId = "ConnectionId"

  @module("spacetimedb") @scope("ConnectionId")
  external fromString: string => connectionId = "fromString"

  @module("spacetimedb") @scope("ConnectionId")
  external random: unit => connectionId = "random"

  @send external toHexString: connectionId => string = "toHexString"
  @send external isEqual: (connectionId, connectionId) => bool = "isEqual"
  @send external equals: (connectionId, connectionId) => bool = "equals"
  @send external isZero: connectionId => bool = "isZero"
  @send external toUint8Array: connectionId => Js.TypedArray2.Uint8Array.t = "toUint8Array"
}

// ═══════════════════════════════════════════════════════════════════════
// Uuid
// ═══════════════════════════════════════════════════════════════════════

@genType
module Uuid = {
  @module("spacetimedb") @scope("Uuid")
  external parse: string => uuid = "parse"

  @send external toString: uuid => string = "toString"
  @send external asBigInt: uuid => bigint = "asBigInt"
  @send external compareTo: (uuid, uuid) => int = "compareTo"
  @send external toBytes: uuid => Js.TypedArray2.Uint8Array.t = "toBytes"

  @module("spacetimedb") @scope("Uuid")
  external nil: uuid = "NIL"

  @module("spacetimedb") @scope("Uuid")
  external max: uuid = "MAX"
}

// ═══════════════════════════════════════════════════════════════════════
// TimeDuration
// ═══════════════════════════════════════════════════════════════════════

@genType
module TimeDuration = {
  @new @module("spacetimedb")
  external fromMicros: bigint => timeDuration = "TimeDuration"

  @module("spacetimedb") @scope("TimeDuration")
  external fromMillis: int => timeDuration = "fromMillis"

  @get external micros: timeDuration => bigint = "micros"
  @get external millis: timeDuration => int = "millis"
  @send external toString: timeDuration => string = "toString"
}

// ═══════════════════════════════════════════════════════════════════════
// ScheduleAt — SDK sum type: Interval(TimeDuration) | Time(Timestamp)
// ═══════════════════════════════════════════════════════════════════════

@genType
@tag("tag")
type scheduleAt =
  | Interval({value: timeDuration})
  | Time({value: timestamp})

@genType
module ScheduleAt = {
  @module("spacetimedb") @scope("ScheduleAt")
  external interval: bigint => scheduleAt = "interval"

  @module("spacetimedb") @scope("ScheduleAt")
  external time: bigint => scheduleAt = "time"
}

// ═══════════════════════════════════════════════════════════════════════
// DbConnectionBuilder — fluent builder for database connections
// ═══════════════════════════════════════════════════════════════════════

// Generic SDK types — @genType.import makes them available for genType resolution,
// but without @genType on the type itself they won't appear directly in .gen.tsx.
// Concrete non-generic aliases are exported from the genType guard file.
@genType.import(("spacetimedb", "DbConnectionBuilder"))
type dbConnectionBuilder<'conn>
@genType.import(("spacetimedb", "DbConnectionImpl"))
type dbConnectionImpl<'rm>
type dbConfig

@new @module("spacetimedb/sdk")
external makeDbConnectionBuilder: ('remoteModule, dbConfig => dbConnectionImpl<'a>) => dbConnectionBuilder<'a> =
  "DbConnectionBuilder"

@new @module("spacetimedb/sdk")
external makeDbConnectionImpl: dbConfig => dbConnectionImpl<'a> = "DbConnectionImpl"

// Builder methods (fluent — each returns the builder)
@send external withUri: (dbConnectionBuilder<'a>, string) => dbConnectionBuilder<'a> = "withUri"
@send external withDatabaseName: (dbConnectionBuilder<'a>, string) => dbConnectionBuilder<'a> = "withDatabaseName"
@send external withToken: (dbConnectionBuilder<'a>, option<string>) => dbConnectionBuilder<'a> = "withToken"
@send
external withCompression: (dbConnectionBuilder<'a>, [#gzip | #none]) => dbConnectionBuilder<'a> = "withCompression"
@send external withLightMode: (dbConnectionBuilder<'a>, bool) => dbConnectionBuilder<'a> = "withLightMode"
@send
external withConfirmedReads: (dbConnectionBuilder<'a>, bool) => dbConnectionBuilder<'a> = "withConfirmedReads"

// Builder callbacks
@send
external onConnect: (
  dbConnectionBuilder<'a>,
  (dbConnectionImpl<'a>, identity, string) => unit,
) => dbConnectionBuilder<'a> = "onConnect"

@send
external onConnectError: (
  dbConnectionBuilder<'a>,
  ('ctx, JsExn.t) => unit,
) => dbConnectionBuilder<'a> = "onConnectError"

@send
external onDisconnect: (
  dbConnectionBuilder<'a>,
  ('ctx, option<JsExn.t>) => unit,
) => dbConnectionBuilder<'a> = "onDisconnect"

// Terminal — builds and returns the connection
@send external build: dbConnectionBuilder<'a> => dbConnectionImpl<'a> = "build"

// ═══════════════════════════════════════════════════════════════════════
// DbConnectionImpl — active connection instance
// ═══════════════════════════════════════════════════════════════════════

@get external isActive: dbConnectionImpl<'a> => bool = "isActive"
@get external connIdentity: dbConnectionImpl<'a> => option<identity> = "identity"
@get external connToken: dbConnectionImpl<'a> => option<string> = "token"
@get external connConnectionId: dbConnectionImpl<'a> => connectionId = "connectionId"
@send external disconnect: dbConnectionImpl<'a> => unit = "disconnect"

// Dynamic property access for db/reducers/procedures views.
// These return framework-specific accessor objects — typed per-project by codegen.
@get external db: dbConnectionImpl<'a> => 'b = "db"
@get external getReducers: dbConnectionImpl<'a> => 'b = "reducers"
@get external getProcedures: dbConnectionImpl<'a> => 'b = "procedures"

// ═══════════════════════════════════════════════════════════════════════
// SubscriptionBuilder — SQL query subscription
// ═══════════════════════════════════════════════════════════════════════

@genType.import(("spacetimedb", "SubscriptionBuilderImpl"))
type subscriptionBuilder<'rm>
@genType.import(("spacetimedb", "SubscriptionHandleImpl"))
type subscriptionHandle<'rm>

@send external subscriptionBuilder: dbConnectionImpl<'a> => subscriptionBuilder<'a> = "subscriptionBuilder"

@send
external onApplied: (subscriptionBuilder<'a>, 'ctx => unit) => subscriptionBuilder<'a> = "onApplied"

@send
external onSubError: (subscriptionBuilder<'a>, ('ctx, JsExn.t) => unit) => subscriptionBuilder<'a> = "onError"

@send
external subscribe: (subscriptionBuilder<'a>, array<string>) => subscriptionHandle<'a> = "subscribe"

@send external subscribeToAllTables: subscriptionBuilder<'a> => unit = "subscribeToAllTables"

// SubscriptionHandle methods
@send external unsubscribe: subscriptionHandle<'a> => unit = "unsubscribe"
@send external unsubscribeThen: (subscriptionHandle<'a>, 'ctx => unit) => unit = "unsubscribeThen"
@send external isSubActive: subscriptionHandle<'a> => bool = "isActive"
@send external isSubEnded: subscriptionHandle<'a> => bool = "isEnded"

// ═══════════════════════════════════════════════════════════════════════
// Query builder
// ═══════════════════════════════════════════════════════════════════════

type queryBuilder<'schema>
type query<'tableDef>
type booleanExpr

@module("spacetimedb/sdk")
external makeQueryBuilder: 'schemaDef => queryBuilder<'schema> = "makeQueryBuilder"

// ═══════════════════════════════════════════════════════════════════════
// convertToAccessorMap — used by codegen for reducer/procedure views
// ═══════════════════════════════════════════════════════════════════════

@module("spacetimedb/sdk")
external convertToAccessorMap: array<'a> => Dict.t<'a> = "convertToAccessorMap"

// ═══════════════════════════════════════════════════════════════════════
// SDK Result bridge — {ok: T} | {err: E} → result<T, E>
// ═══════════════════════════════════════════════════════════════════════

type sdkResult<'ok, 'err>

@val external _hasOwn: ('a, string) => bool = "Object.hasOwn"
@get external _rawOk: sdkResult<'ok, 'err> => 'ok = "ok"
@get external _rawErr: sdkResult<'ok, 'err> => 'err = "err"

let fromSdkResult = (raw: sdkResult<'ok, 'err>): result<'ok, 'err> =>
  if _hasOwn(raw, "ok") {
    Ok(_rawOk(raw))
  } else {
    Error(_rawErr(raw))
  }

// ═══════════════════════════════════════════════════════════════════════
// React hooks — SDK-native React integration
// ═══════════════════════════════════════════════════════════════════════

@genType
module React = {
  @module("spacetimedb/react") @react.component
  external spacetimeDBProvider: (
    ~connectionBuilder: dbConnectionBuilder<'a>,
    ~children: React.element,
  ) => React.element = "SpacetimeDBProvider"

  @module("spacetimedb/react")
  external useTable: query<'tableDef> => (array<'row>, bool) = "useTable"

  @module("spacetimedb/react")
  external useReducer: 'reducerDef => 'callFn = "useReducer"

  type connectionState
  @module("spacetimedb/react")
  external useSpacetimeDB: unit => connectionState = "useSpacetimeDB"
}

// ═══════════════════════════════════════════════════════════════════════
// Utilities
// ═══════════════════════════════════════════════════════════════════════

@val @scope("Promise")
external promiseRace: array<promise<'a>> => promise<'a> = "race"

@val external setTimeout: (unit => unit, int) => float = "setTimeout"


