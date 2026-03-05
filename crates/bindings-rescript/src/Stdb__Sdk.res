// Stdb__Sdk.res — Single source of truth for all SpacetimeDB SDK types and bindings.
//
// This file replaces the scattered type declarations across Stdb.Types preamble,
// StdbServerConnection externals, and SpacetimeDBSdk facade.
//
// KEY INSIGHT: The SDK's AlgebraicType is pure data — plain tagged-union JS objects
// like {tag: "U64"}, {tag: "Product", value: {elements: [...]}}. No classes, no
// prototypes, no instanceof checks in the consumption path. ReScript constructs
// these shapes natively via @tag("tag") variants.

// ─── Opaque SDK types ───────────────────────────────────────────────

type connection
type eventCtx
type reducers
type procedures

// ─── Identity ───────────────────────────────────────────────────────

type identity // opaque — SDK _Identity class instance
@send external identityToHex: identity => string = "toHexString"
@send external identityToString: identity => string = "toString"
@send external identityIsEqual: (identity, identity) => bool = "isEqual"

module Identity = {
  @module("spacetimedb") @scope("Identity")
  external fromString: string => identity = "fromString"
  let toHex = identityToHex
  let toString = identityToString
  let isEqual = identityIsEqual
}

// ─── ConnectionId ───────────────────────────────────────────────────

type connectionId // opaque — SDK _ConnectionId class instance
@send external connectionIdToHex: connectionId => string = "toHexString"
@send external connectionIdIsEqual: (connectionId, connectionId) => bool = "isEqual"

module ConnectionId = {
  @module("spacetimedb") @scope("ConnectionId")
  external fromString: string => connectionId = "fromString"
  let toHex = connectionIdToHex
  let isEqual = connectionIdIsEqual
}

// ─── Uuid ───────────────────────────────────────────────────────────

type uuid // opaque — SDK _Uuid class instance
@send external uuidToString: uuid => string = "toString"

module Uuid = {
  @module("spacetimedb") @scope("Uuid")
  external parse: string => uuid = "parse"
  let toString = uuidToString
}

// ─── Timestamp ──────────────────────────────────────────────────────

type timestamp // opaque — SDK Timestamp class instance
@send external timestampToMillis: (timestamp) => bigint = "toMillis"
@send external timestampToDate: (timestamp) => Date.t = "toDate"
let timestampToFloatMs = (ts: timestamp): float => ts->timestampToMillis->BigInt.toFloat

// ─── TimeDuration ───────────────────────────────────────────────────

type timeDuration // opaque — SDK TimeDuration class instance
@send external timeDurationToMicros: (timeDuration) => bigint = "toMicros"

// ─── ScheduleAt ─────────────────────────────────────────────────────

@tag("tag")
type scheduleAt =
  | Interval({value: timeDuration})
  | Time({value: timestamp})

// ─── AlgebraicType — two-tier @unboxed + @tag("tag") design ─────────
//
// @unboxed compiles primitives to bare strings ("U8", "Bool", "U64").
// The SDK expects {tag: "U64"} objects. The js_exports.mjs shim
// normalizes bare strings → {tag: str} before the SDK sees them.
// Compound types (Product/Sum/Array/Ref) already compile to {tag: "...", value: ...}
// via @tag("tag") on compoundType, so they pass through unchanged.

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

// ─── REMOTE_MODULE data types ───────────────────────────────────────
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
// Used as tableDef.tableDef property. We type it as opaque since
// DbConnectionImpl only reads our higher-level tableDef properties,
// not the raw V10 shape directly.
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

// ─── Schema assembly helpers (called from generated StdbSchema.res) ──

// makeQueryBuilder: takes {tables: Dict.t<tableDef>} and returns a frozen
// object with one property per table accessorName, each being a TableRef
// usable by React hooks (useTable, etc.)
type schemaDef = {tables: Dict.t<tableDef>}
type tableQueries<'a> = 'a // phantom — concrete type is generated per-project

@module("spacetimedb")
external makeQueryBuilder: schemaDef => tableQueries<'a> = "makeQueryBuilder"

// convertToAccessorMap: takes array<reducerDef> and returns a record keyed
// by accessorName. Used by reducer React hooks.
type reducerAccessors<'a> = 'a // phantom — concrete type is generated per-project

@module("spacetimedb")
external convertToAccessorMap: array<reducerDef> => reducerAccessors<'a> = "convertToAccessorMap"

// ─── DbConnection builder externals ─────────────────────────────────

type dbConnectionBuilder
type dbConfig
type dbConnectionImpl

@new @module("@spacetimedb/rescript/src/js_exports.mjs")
external makeDbConnectionBuilder: (remoteModule, dbConfig => dbConnectionImpl) => dbConnectionBuilder =
  "DbConnectionBuilder"

@new @module("spacetimedb/sdk")
external makeDbConnectionImpl: dbConfig => dbConnectionImpl = "DbConnectionImpl"

@send
external withUri: (dbConnectionBuilder, string) => dbConnectionBuilder = "withUri"

@send
external withDatabaseName: (dbConnectionBuilder, string) => dbConnectionBuilder = "withDatabaseName"

@send
external withToken: (dbConnectionBuilder, option<string>) => dbConnectionBuilder = "withToken"

@send
external onConnect: (dbConnectionBuilder, (connection, string, string) => unit) => dbConnectionBuilder =
  "onConnect"

@send
external onConnectError: (dbConnectionBuilder, ('ctx, JsExn.t) => unit) => dbConnectionBuilder =
  "onConnectError"

@send
external buildConnection: dbConnectionBuilder => connection = "build"

// ─── Connection instance methods ────────────────────────────────────

@get external isActive: connection => bool = "isActive"
@send external disconnect: connection => unit = "disconnect"

// ─── Subscription builder ───────────────────────────────────────────

type subscriptionBuilder

@send
external subscriptionBuilder: connection => subscriptionBuilder = "subscriptionBuilder"

@send
external onApplied: (subscriptionBuilder, unit => unit) => subscriptionBuilder = "onApplied"

@send
external onSubError: (subscriptionBuilder, ('ctx, JsExn.t) => unit) => subscriptionBuilder =
  "onError"

@send
external subscribe: (subscriptionBuilder, array<string>) => unit = "subscribe"

// ─── Utilities ──────────────────────────────────────────────────────

// Connection builder — opaque type for SpacetimeDBProvider
type connectionBuilder = dbConnectionBuilder

// Promise.race for timeout support
@val @scope("Promise")
external promiseRace: array<promise<'a>> => promise<'a> = "race"

@val external setTimeout: (unit => unit, int) => float = "setTimeout"
