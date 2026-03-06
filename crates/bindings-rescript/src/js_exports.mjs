// Re-export everything from spacetimedb unchanged
export * from 'spacetimedb';

// Override DbConnectionBuilder with normalizing wrapper.
// ReScript's @unboxed algebraicType compiles primitives to bare strings ("U64"),
// but the SDK expects {tag: "U64"} objects. This shim normalizes the remoteModule
// at the DbConnectionBuilder boundary so the SDK always sees tagged objects.
import { DbConnectionBuilder as _DbConnectionBuilder } from 'spacetimedb/sdk';

const kProcedureWrapped = Symbol('rescriptProcedureWrapped');
const kProceduresPatched = Symbol('rescriptProceduresPatched');

function normalizeProcedureResult(value) {
  if (value == null || typeof value !== 'object') return value;
  if (Object.prototype.hasOwnProperty.call(value, 'TAG')) return value;
  if (Object.prototype.hasOwnProperty.call(value, 'ok')) {
    return { TAG: 'Ok', _0: value.ok };
  }
  if (Object.prototype.hasOwnProperty.call(value, 'err')) {
    return { TAG: 'Error', _0: value.err };
  }
  return value;
}

function patchProcedures(procedures) {
  if (!procedures || typeof procedures !== 'object' || procedures[kProceduresPatched]) {
    return procedures;
  }

  for (const key of Object.keys(procedures)) {
    const original = procedures[key];
    if (typeof original !== 'function' || original[kProcedureWrapped]) continue;

    const wrapped = function (...args) {
      return Promise.resolve(original.apply(this, args)).then(normalizeProcedureResult);
    };
    wrapped[kProcedureWrapped] = true;
    procedures[key] = wrapped;
  }

  procedures[kProceduresPatched] = true;
  return procedures;
}

function patchConnectionProcedures(connection) {
  if (!connection || typeof connection !== 'object') return connection;
  patchProcedures(connection.procedures);
  return connection;
}

function wrapConfigFn(configFn) {
  if (typeof configFn !== 'function') return configFn;
  return function (...args) {
    const connection = configFn.apply(this, args);
    if (connection && typeof connection.then === 'function') {
      return connection.then(patchConnectionProcedures);
    }
    return patchConnectionProcedures(connection);
  };
}

function normalizeAlgType(ty) {
  if (typeof ty === 'string') return { tag: ty };
  if (ty.tag === 'Product' && ty.value?.elements) {
    ty.value.elements = ty.value.elements.map(e => ({
      ...e, algebraicType: normalizeAlgType(e.algebraicType)
    }));
  } else if (ty.tag === 'Sum' && ty.value?.variants) {
    ty.value.variants = ty.value.variants.map(v => ({
      ...v, algebraicType: normalizeAlgType(v.algebraicType)
    }));
  } else if (ty.tag === 'Array') {
    ty.value = normalizeAlgType(ty.value);
  }
  // Ref: {tag: "Ref", value: int} — no nested algebraicType
  return ty;
}

function normalizeProductType(pt) {
  if (!pt?.elements) return pt;
  return {
    ...pt,
    elements: pt.elements.map(e => ({
      ...e, algebraicType: normalizeAlgType(e.algebraicType)
    }))
  };
}

// Convert raw ProductType { elements: [{ name, algebraicType }, ...] }
// into the TypeBuilder-dict format { fieldName: { algebraicType }, ... }
// that the SDK's ProductBuilder constructor expects for procedure params.
function productTypeToTypeBuilderDict(pt) {
  if (!pt?.elements || !Array.isArray(pt.elements)) return pt;
  const dict = {};
  for (const elem of pt.elements) {
    dict[elem.name] = { algebraicType: normalizeAlgType(elem.algebraicType) };
  }
  return dict;
}

function normalizeRemoteModule(rm) {
  for (const table of Object.values(rm.tables)) {
    table.rowType = normalizeProductType(table.rowType);
    if (table.columns) {
      for (const col of Object.values(table.columns)) {
        if (col.typeBuilder?.algebraicType) {
          col.typeBuilder.algebraicType = normalizeAlgType(col.typeBuilder.algebraicType);
        }
      }
    }
  }
  for (const reducer of rm.reducers) {
    reducer.paramsType = normalizeProductType(reducer.paramsType);
  }
  for (const procedure of rm.procedures) {
    // SDK's DbConnectionImpl wraps procedure.params in ProductBuilder(params),
    // which expects { fieldName: { algebraicType }, ... } not { elements: [...] }.
    procedure.params = productTypeToTypeBuilderDict(procedure.params);
    if (procedure.returnType?.algebraicType) {
      procedure.returnType.algebraicType = normalizeAlgType(procedure.returnType.algebraicType);
    }
  }
  return rm;
}

export class DbConnectionBuilder extends _DbConnectionBuilder {
  constructor(remoteModule, configFn) {
    super(normalizeRemoteModule(remoteModule), wrapConfigFn(configFn));
  }
}
