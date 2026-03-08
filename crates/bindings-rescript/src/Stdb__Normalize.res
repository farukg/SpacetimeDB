// Stdb__Normalize.res — Typed normalization of ReScript algebraicType values
// for the SpacetimeDB SDK.
//
// PROBLEM: ReScript's @unboxed algebraicType compiles primitives to bare strings
// ("U64", "Bool"), but the SDK expects {tag: "U64"} tagged objects.
// Compound types (Product/Sum/Array/Ref) already compile to {tag: "...", value: ...}
// via @tag("tag") on compoundType, so they pass through unchanged.
//
// SOLUTION: This module provides typed normalization functions that convert the
// ReScript representation to the SDK's expected shapes. All functions are pure
// ReScript — no %raw. The only escape hatch is a single %identity at the
// primitive FFI boundary, justified per §MOD.5e.
//
// ARCHITECTURE: This module is the ONLY place where representation normalization
// happens. It replaces the hand-written JS normalizeAlgType, normalizeProductType,
// productTypeToTypeBuilderDict, and normalizeRemoteModule from js_exports.mjs.

open Stdb__Sdk

// ─── SDK-expected opaque types ──────────────────────────────────────
//
// The SDK consumes these shapes — they are opaque from ReScript's perspective.
// We construct them at the boundary and never inspect them again.

type sdkAlgebraicType

// {tag: "U64"} — the shape SDK expects for primitives
@obj external makeSdkPrimitive: (~tag: string) => sdkAlgebraicType = ""

// For compound types that already have the correct {tag, value} shape,
// we just need to recurse into children and pass through.
// SIG §MOD.5e: identity cast at FFI boundary — compound types are already
// {tag: "Product", value: {elements: [...]}} objects. After recursing into
// children to normalize nested algebraicTypes, we cast back to sdkAlgebraicType.
external compoundToSdk: 'a => sdkAlgebraicType = "%identity"

// ─── Primitive extraction ───────────────────────────────────────────
//
// SIG §MOD.5e: @unboxed payloadless variants ARE strings at runtime.
// The switch dispatch in toSdkAlgType guarantees the primitive arm only
// matches string values. This %identity treats the opaque algebraicType
// as the string it actually is at runtime.
external primitiveToString: algebraicType => string = "%identity"

// ─── SDK element/variant types (post-normalization) ─────────────────

// sdkProductElement and sdkSumVariant share the same shape — unified as sdkElement
type sdkElement = {name: option<string>, algebraicType: sdkAlgebraicType}
type sdkProductType = {elements: array<sdkElement>}
type sdkSumType = {variants: array<sdkElement>}

// ─── Core normalization ─────────────────────────────────────────────

let rec toSdkAlgType = (ty: algebraicType): sdkAlgebraicType =>
  switch ty {
  | Compound(c) => normalizeSdkCompound(c)
  | primitive => makeSdkPrimitive(~tag=primitiveToString(primitive))
  }

and normalizeSdkCompound = (c: compoundType<algebraicType>): sdkAlgebraicType =>
  switch c {
  | Product({value: pt}) =>
    let elements = pt.elements->Array.map(e => {
      name: e.name,
      algebraicType: toSdkAlgType(e.algebraicType),
    })
    compoundToSdk({"tag": "Product", "value": {"elements": elements}})
  | Sum({value: st}) =>
    let variants = st.variants->Array.map(v => {
      name: v.name,
      algebraicType: toSdkAlgType(v.algebraicType),
    })
    compoundToSdk({"tag": "Sum", "value": {"variants": variants}})
  | Array({value: inner}) =>
    compoundToSdk({"tag": "Array", "value": toSdkAlgType(inner)})
  | Ref({value: n}) =>
    compoundToSdk({"tag": "Ref", "value": n})
  }

// ─── ProductType normalization ──────────────────────────────────────

let normalizeSdkProductType = (pt: productType<algebraicType>): sdkProductType => {
  elements: pt.elements->Array.map(e => {
    name: e.name,
    algebraicType: toSdkAlgType(e.algebraicType),
  }),
}

// ─── TypeBuilder dict conversion ────────────────────────────────────
//
// Converts productType {elements: [{name, algebraicType}, ...]}
// into the dict format {fieldName: {algebraicType}} that the SDK's
// ProductBuilder constructor expects for procedure params.

type sdkTypeBuilderLike = {algebraicType: sdkAlgebraicType}

let productTypeToTypeBuilderDict = (pt: productType<algebraicType>): Dict.t<
  sdkTypeBuilderLike,
> =>
  pt.elements
  ->Array.map(elem => (
    elem.name->Option.getOr(""),
    {algebraicType: toSdkAlgType(elem.algebraicType)},
  ))
  ->Dict.fromArray

// ─── Column normalization ───────────────────────────────────────────

// SDK column defs have a typeBuilder.algebraicType that needs normalization.
// We reconstruct the column with the normalized algebraicType.
type sdkColumnDef = {
  columnMetadata: columnMetadata,
  typeBuilder: sdkTypeBuilderLike,
}

let normalizeColumnDef = (col: columnDef): sdkColumnDef => {
  columnMetadata: col.columnMetadata,
  typeBuilder: {algebraicType: toSdkAlgType(col.typeBuilder.algebraicType)},
}

// ─── RemoteModule normalization ─────────────────────────────────────
//
// This is the top-level function that replaces normalizeRemoteModule from
// the JS shim. It normalizes all algebraicType references in tables,
// reducers, and procedures to the SDK-expected format.

// SDK-expected shapes (post-normalization)
type sdkTableDef = {
  sourceName: string,
  accessorName: string,
  rowType: sdkProductType,
  columns: Dict.t<sdkColumnDef>,
  indexes: array<indexDef>,
  constraints: array<constraintDef>,
  tableDef?: rawTableDefV10,
  isEvent?: bool,
}

type sdkReducerDef = {
  name: string,
  accessorName: string,
  paramsType: sdkProductType,
}

type sdkProcedureDef = {
  name: string,
  accessorName: string,
  params: Dict.t<sdkTypeBuilderLike>,
  returnType: sdkTypeBuilderLike,
}

type sdkRemoteModule = {
  versionInfo: versionInfo,
  tables: Dict.t<sdkTableDef>,
  reducers: array<sdkReducerDef>,
  procedures: array<sdkProcedureDef>,
}

let normalizeRemoteModule = (rm: remoteModule): sdkRemoteModule => {
  versionInfo: rm.versionInfo,
  tables: rm.tables->Dict.toArray->Array.map(((key, table)) => (
    key,
    {
      sourceName: table.sourceName,
      accessorName: table.accessorName,
      rowType: normalizeSdkProductType(table.rowType),
      columns: table.columns->Dict.toArray->Array.map(((colKey, col)) => (
        colKey,
        normalizeColumnDef(col),
      ))->Dict.fromArray,
      indexes: table.indexes,
      constraints: table.constraints,
      tableDef: ?table.tableDef,
      isEvent: ?table.isEvent,
    },
  ))->Dict.fromArray,
  reducers: rm.reducers->Array.map(reducer => {
    name: reducer.name,
    accessorName: reducer.accessorName,
    paramsType: normalizeSdkProductType(reducer.paramsType),
  }),
  procedures: rm.procedures->Array.map(procedure => {
    name: procedure.name,
    accessorName: procedure.accessorName,
    params: productTypeToTypeBuilderDict(procedure.params),
    returnType: {algebraicType: toSdkAlgType(procedure.returnType.algebraicType)},
  }),
}

// ─── DbConnectionBuilder ────────────────────────────────────────────
//
// Direct import from SDK, with normalization applied at construction.
// Replaces the JS class that extended _DbConnectionBuilder.

@new @module("spacetimedb/sdk")
external _makeRawBuilder: (sdkRemoteModule, dbConfig => dbConnectionImpl<'a>) => dbConnectionBuilder<'a> =
  "DbConnectionBuilder"

let makeNormalizedBuilder = (rm: remoteModule, configFn: dbConfig => dbConnectionImpl<'a>) =>
  _makeRawBuilder(normalizeRemoteModule(rm), configFn)
