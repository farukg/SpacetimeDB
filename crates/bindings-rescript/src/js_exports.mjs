// Re-export everything from spacetimedb unchanged
export * from 'spacetimedb';

// Override DbConnectionBuilder with normalizing wrapper.
// ReScript's @unboxed algebraicType compiles primitives to bare strings ("U64"),
// but the SDK expects {tag: "U64"} objects. This shim normalizes the remoteModule
// at the DbConnectionBuilder boundary so the SDK always sees tagged objects.
import { DbConnectionBuilder as _DbConnectionBuilder } from 'spacetimedb/sdk';

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
    procedure.params = normalizeProductType(procedure.params);
    if (procedure.returnType?.algebraicType) {
      procedure.returnType.algebraicType = normalizeAlgType(procedure.returnType.algebraicType);
    }
  }
  return rm;
}

export class DbConnectionBuilder extends _DbConnectionBuilder {
  constructor(remoteModule, configFn) {
    super(normalizeRemoteModule(remoteModule), configFn);
  }
}
