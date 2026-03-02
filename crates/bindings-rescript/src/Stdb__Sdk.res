// Stdb__Sdk.res — Single source of truth for all SpacetimeDB SDK types and bindings.
//
// This file replaces the scattered type declarations across StdbTypes preamble,
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

// ─── AlgebraicType — pure data matching SDK's tagged-union shape ────
//
// The SDK dispatches on `ty.tag` via string comparison:
//   switch (ty.tag) { case "Product": ... case "Sum": ... default: primitiveSerializers[ty.tag] }
//
// Our @tag("tag") variants compile to identical JS objects:
//   U64 → {tag: "U64"}
//   Product({value: ...}) → {tag: "Product", value: ...}

// The SDK dispatches on `ty.tag` via `switch (ty.tag)` in makeDeserializer/
// makeSerializer. ALL algebraic type values MUST be objects with a `.tag`
// property — bare strings like "U64" fail because `"U64".tag === undefined`.
//
// ReScript v12 compiles payloadless @tag("tag") variants to bare strings,
// so primitives are constructed as `{tag: "U64"}` objects via external casts
// in the AlgType module below. Compound types (Product, Sum, Array, Ref)
// naturally have payloads and compile correctly as `{tag: "Product", value: ...}`.

@tag("tag")
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
  | @as("String") String_
  | @as("Product") Product({value: productType})
  | @as("Sum") Sum({value: sumType})
  | @as("Array") Array_({value: algebraicType})
  | @as("Ref") Ref({value: int})
and productType = {elements: array<productElement>}
and productElement = {
  name: option<string>,
  algebraicType: algebraicType,
}
and sumType = {variants: array<sumVariant>}
and sumVariant = {
  name: option<string>,
  algebraicType: algebraicType,
}

// ─── AlgType constructors ───────────────────────────────────────────
//
// Convenience constructors for codegen to call.
// These produce the exact JS shapes the SDK expects.

module AlgType = {
  // Primitives — must be {tag: "U8"} objects, NOT bare strings.
  // ReScript v12 compiles payloadless variants to bare strings which breaks
  // the SDK's `switch (ty.tag)` dispatch. We use tagged object literals.
  type tagOnly = {tag: string}
  external asAlgType: tagOnly => algebraicType = "%identity"

  let u8: algebraicType = asAlgType({tag: "U8"})
  let u16: algebraicType = asAlgType({tag: "U16"})
  let u32: algebraicType = asAlgType({tag: "U32"})
  let u64: algebraicType = asAlgType({tag: "U64"})
  let i8: algebraicType = asAlgType({tag: "I8"})
  let i16: algebraicType = asAlgType({tag: "I16"})
  let i32: algebraicType = asAlgType({tag: "I32"})
  let i64: algebraicType = asAlgType({tag: "I64"})
  let u128: algebraicType = asAlgType({tag: "U128"})
  let u256: algebraicType = asAlgType({tag: "U256"})
  let i128: algebraicType = asAlgType({tag: "I128"})
  let i256: algebraicType = asAlgType({tag: "I256"})
  let f32: algebraicType = asAlgType({tag: "F32"})
  let f64: algebraicType = asAlgType({tag: "F64"})
  let bool_: algebraicType = asAlgType({tag: "Bool"})
  let string_: algebraicType = asAlgType({tag: "String"})

  // Compound types
  let product = (elements): algebraicType =>
    Product({value: {elements: elements}})

  let sum = (variants): algebraicType =>
    Sum({value: {variants: variants}})

  let array_ = (inner): algebraicType =>
    Array_({value: inner})

  let ref_ = (index): algebraicType =>
    Ref({value: index})

  // Unit type — Product with no elements
  let unit_ = Product({value: {elements: []}})

  // Option — Sum with "some" and "none" variants (SDK convention)
  let option = (inner): algebraicType =>
    Sum({
      value: {
        variants: [
          {name: Some("some"), algebraicType: inner},
          {name: Some("none"), algebraicType: Product({value: {elements: []}})},
        ],
      },
    })

  // Named element constructors for products/sums
  let element = (~name, ~algebraicType): productElement => {
    name: Some(name),
    algebraicType,
  }

  let variant = (~name, ~algebraicType): sumVariant => {
    name: Some(name),
    algebraicType,
  }

  // Unnamed element (positional, rare)
  let unnamedElement = (~algebraicType): productElement => {
    name: None,
    algebraicType,
  }

  // Special SDK types — exact algebraicType shapes for BSATN serialization.
  // These match the SDK's Identity.getAlgebraicType(), Timestamp.getAlgebraicType(), etc.
  let identity = product([element(~name="__identity__", ~algebraicType=u256)])
  let connectionId = product([element(~name="__connection_id__", ~algebraicType=u128)])
  let timestamp = product([element(~name="__timestamp_micros_since_unix_epoch__", ~algebraicType=i64)])
  let timeDuration = product([element(~name="__time_duration_micros__", ~algebraicType=i64)])
  let uuid = product([element(~name="__uuid__", ~algebraicType=u128)])
  let scheduleAt = sum([
    variant(~name="Interval", ~algebraicType=timeDuration),
    variant(~name="Time", ~algebraicType=timestamp),
  ])

  // Result(ok, err) — Sum with "ok" and "err" variants
  let result = (ok, err): algebraicType =>
    Sum({
      value: {
        variants: [
          {name: Some("ok"), algebraicType: ok},
          {name: Some("err"), algebraicType: err},
        ],
      },
    })

  // ByteArray — shorthand for Array(U8)
  let byteArray = array_(u8)
}

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
  rowType: productType,
  columns: Dict.t<columnDef>,
  indexes: array<indexDef>,
  constraints: array<constraintDef>,
  tableDef?: rawTableDefV10,
  isEvent?: bool,
}

type reducerDef = {
  name: string,
  accessorName: string,
  paramsType: productType,
}

type procedureDef = {
  name: string,
  accessorName: string,
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
