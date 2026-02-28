// StdbSdk.res — Single source of truth for all SpacetimeDB SDK types and bindings.
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

// ReScript v12 compiles payloadless @tag("tag") variants to bare strings
// ("U64"), not objects ({tag: "U64"}). The SDK's getTag() helper handles
// both forms, so primitives can use clean payloadless variants.

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
  // Primitives — bare strings "U8", "U64", etc. SDK's getTag() extracts tag.
  let u8 = U8
  let u16 = U16
  let u32 = U32
  let u64 = U64
  let i8 = I8
  let i16 = I16
  let i32 = I32
  let i64 = I64
  let u128 = U128
  let u256 = U256
  let i128 = I128
  let i256 = I256
  let f32 = F32
  let f64 = F64
  let bool_ = Bool
  let string_ = String_

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
  constraint: string,
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
