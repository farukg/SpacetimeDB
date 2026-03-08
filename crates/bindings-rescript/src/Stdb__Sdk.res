// Stdb__Sdk.res — AlgebraicType definitions and remoteModule structural types.
//
// This file defines OUR type system for schema codegen:
// - algebraicType (two-tier @unboxed + @tag("tag"))
// - remoteModule, tableDef, reducerDef, procedureDef
// - columnMetadata, columnDef
//
// SDK interop externals (Identity, Timestamp, DbConnectionBuilder, etc.)
// live in Stdb__SdkBindings.res — the single §MOD.5b trust boundary.

// ═══════════════════════════════════════════════════════════════════════
// Backward-compat type aliases — re-export SDK types
// ═══════════════════════════════════════════════════════════════════════

type identity = Stdb__SdkBindings.identity
type connectionId = Stdb__SdkBindings.connectionId
type timestamp = Stdb__SdkBindings.timestamp
type timeDuration = Stdb__SdkBindings.timeDuration
type uuid = Stdb__SdkBindings.uuid
type scheduleAt = Stdb__SdkBindings.scheduleAt
type connection<'a> = Stdb__SdkBindings.dbConnectionImpl<'a>
type dbConnectionBuilder<'a> = Stdb__SdkBindings.dbConnectionBuilder<'a>
type dbConnectionImpl<'a> = Stdb__SdkBindings.dbConnectionImpl<'a>
type dbConfig = Stdb__SdkBindings.dbConfig
type subscriptionBuilder<'a> = Stdb__SdkBindings.subscriptionBuilder<'a>
type subscriptionHandle<'a> = Stdb__SdkBindings.subscriptionHandle<'a>
type sdkResult<'ok, 'err> = Stdb__SdkBindings.sdkResult<'ok, 'err>

// Backward-compat module aliases
module Identity = Stdb__SdkBindings.Identity
module ConnectionId = Stdb__SdkBindings.ConnectionId
module Uuid = Stdb__SdkBindings.Uuid
module Timestamp = Stdb__SdkBindings.Timestamp
module TimeDuration = Stdb__SdkBindings.TimeDuration
module ScheduleAt = Stdb__SdkBindings.ScheduleAt

// Backward-compat value aliases
let identityToHex = Stdb__SdkBindings.Identity.toHexString
let identityToString = Stdb__SdkBindings.Identity.toString
let identityIsEqual = Stdb__SdkBindings.Identity.isEqual
let connectionIdToHex = Stdb__SdkBindings.ConnectionId.toHexString
let connectionIdIsEqual = Stdb__SdkBindings.ConnectionId.isEqual
let uuidToString = Stdb__SdkBindings.Uuid.toString
let timestampToMillis = Stdb__SdkBindings.Timestamp.toMillis
let timestampToDate = Stdb__SdkBindings.Timestamp.toDate
let timestampToFloatMs = Stdb__SdkBindings.Timestamp.toFloatMs
let timeDurationToMicros = Stdb__SdkBindings.TimeDuration.micros

// Connection builder re-exports
let makeDbConnectionImpl = Stdb__SdkBindings.makeDbConnectionImpl
let withUri = Stdb__SdkBindings.withUri
let withDatabaseName = Stdb__SdkBindings.withDatabaseName
let withToken = Stdb__SdkBindings.withToken
let onConnect = Stdb__SdkBindings.onConnect
let onConnectError = Stdb__SdkBindings.onConnectError
let buildConnection = Stdb__SdkBindings.build

// Connection instance re-exports
let isActive = Stdb__SdkBindings.isActive
let disconnect = Stdb__SdkBindings.disconnect
let getReducers = Stdb__SdkBindings.getReducers
let getProcedures = Stdb__SdkBindings.getProcedures

// Subscription re-exports
let subscriptionBuilder = Stdb__SdkBindings.subscriptionBuilder
let onApplied = Stdb__SdkBindings.onApplied
let onSubError = Stdb__SdkBindings.onSubError
let subscribe = Stdb__SdkBindings.subscribe

// SDK Result bridge
let fromSdkResult = Stdb__SdkBindings.fromSdkResult

// Utilities
let promiseRace = Stdb__SdkBindings.promiseRace
let setTimeout = Stdb__SdkBindings.setTimeout

// Opaque types used by codegen templates
type eventCtx
type reducers
type procedures

// ═══════════════════════════════════════════════════════════════════════
// AlgebraicType — two-tier @unboxed + @tag("tag") design
// ═══════════════════════════════════════════════════════════════════════
//
// @unboxed compiles primitives to bare strings ("U8", "Bool", "U64").
// The SDK expects {tag: "U64"} tagged objects. Stdb__Normalize.res
// normalizes bare strings → {tag: str} before the SDK sees them.
// Compound types (Product/Sum/Array/Ref) already compile to
// {tag: "...", value: ...} via @tag("tag") on compoundType.

// Supporting types — parameterized to break mutual recursion
type productElement<'a> = {name: option<string>, algebraicType: 'a}
type productType<'a> = {elements: array<productElement<'a>>}
type sumVariant<'a> = {name: option<string>, algebraicType: 'a}
type sumType<'a> = {variants: array<sumVariant<'a>>}

// Compound wrapper — @tag("tag"), parameterized, NOT recursive
@tag("tag")
type compoundType<'a> =
  | Product({value: productType<'a>})
  | Sum({value: sumType<'a>})
  | Array({value: 'a})
  | Ref({value: int})

// algebraicType — @unboxed, self-recursive only via Compound
@unboxed
type rec algebraicType =
  | U8
  | U16
  | U32
  | U64
  | I8
  | I16
  | I32
  | I64
  | U128
  | U256
  | I128
  | I256
  | F32
  | F64
  | Bool
  | String
  | Compound(compoundType<algebraicType>)

// ═══════════════════════════════════════════════════════════════════════
// REMOTE_MODULE data types
// ═══════════════════════════════════════════════════════════════════════
//
// These match the SDK's duck-typed shapes consumed by DbConnectionImpl.
// No class instances needed — all property reads, no instanceof.

type columnMetadata = {
  isPrimaryKey?: bool,
  isUnique?: bool,
  isAutoIncrement?: bool,
  name?: string,
}

type typeBuilderLike = {algebraicType: algebraicType}

type columnDef = {
  columnMetadata: columnMetadata,
  typeBuilder: typeBuilderLike,
}

// RawTableDefV10 — SDK's internal BSATN table definition.
type rawTableDefV10

type indexDef = {
  name: string,
  algorithm: string,
  columns: array<string>,
}

type constraintDef = {
  constraintName: string,
  @as("constraint") constraint_: string,
  columns: array<string>,
}

type tableDef = {
  sourceName: string,
  accessorName: string,
  rowType: productType<algebraicType>,
  columns: Dict.t<columnDef>,
  indexes: array<indexDef>,
  constraints: array<constraintDef>,
  tableDef?: rawTableDefV10,
  isEvent?: bool,
}

type reducerDef = {
  name: string,
  accessorName: string,
  paramsType: productType<algebraicType>,
}

type procedureDef = {
  name: string,
  accessorName: string,
  params: productType<algebraicType>,
  returnType: typeBuilderLike,
}

type versionInfo = {cliVersion: string}

type remoteModule = {
  versionInfo: versionInfo,
  tables: Dict.t<tableDef>,
  reducers: array<reducerDef>,
  procedures: array<procedureDef>,
}

// ═══════════════════════════════════════════════════════════════════════
// Schema assembly helpers (called from generated Stdb__Schema.res)
// ═══════════════════════════════════════════════════════════════════════

type schemaDef = {tables: Dict.t<tableDef>}
type tableQueries<'a> = 'a

@module("spacetimedb")
external makeQueryBuilder: schemaDef => tableQueries<'a> = "makeQueryBuilder"

type reducerAccessors<'a> = 'a

@module("spacetimedb")
external convertToAccessorMap: array<reducerDef> => reducerAccessors<'a> = "convertToAccessorMap"

// Backward-compat alias
type connectionBuilder<'a> = dbConnectionBuilder<'a>
