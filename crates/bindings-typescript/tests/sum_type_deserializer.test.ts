import { describe, expect, test } from 'vitest';
import { AlgebraicType, ProductType, SumType } from '../src/lib/algebraic_type';
import BinaryReader from '../src/lib/binary_reader';
import BinaryWriter from '../src/lib/binary_writer';

// Regression coverage for the 2026-04-18 SumType.makeDeserializer rewrite
// that aligns emission to ReScript's @tag("tag") pattern shape:
//   - plain enums (all variants payloadless) → bare PascalCase strings
//   - mixed sums: unit variants → bare PascalCase string,
//                 payload variants → { tag, value, _0 }
//   - option/result → special-cased direct payload / {ok|err: value}
//
// Without these shapes, downstream ReScript pattern matches crash with
// `.toString()` errors on variant payload newtypes (documented in
// sigma/context-modules.d/memory/products/d-diego/stdb-rescript-normalize-bottleneck-2026-04-17.audit.md).

const emptyProduct = AlgebraicType.Product({ elements: [] });

const roundTrip = (
  ty: ReturnType<typeof AlgebraicType.Sum>,
  // serialize-in value — uses the same builder API codegen emits
  write: (writer: BinaryWriter) => void
) => {
  const writer = new BinaryWriter(8);
  write(writer);
  const reader = new BinaryReader(writer.getBuffer());
  return SumType.makeDeserializer(ty.value)(reader);
};

describe('SumType.makeDeserializer', () => {
  test('plain enum fast path emits bare PascalCase-preserving strings', () => {
    // Note the fast path uses the VARIANT NAME as-is (no capitalisation),
    // since ReScript polymorphic variants compile to the exact string.
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'Queued', algebraicType: emptyProduct },
        { name: 'Running', algebraicType: emptyProduct },
        { name: 'Done', algebraicType: emptyProduct },
      ],
    });
    const deser = SumType.makeDeserializer(ty.value);
    for (let i = 0; i < 3; i++) {
      const writer = new BinaryWriter(1);
      writer.writeByte(i);
      const reader = new BinaryReader(writer.getBuffer());
      expect(deser(reader)).toEqual(ty.value.variants[i].name);
    }
  });

  test('plain enum fast path throws TypeError on unknown tag', () => {
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'A', algebraicType: emptyProduct },
        { name: 'B', algebraicType: emptyProduct },
      ],
    });
    const deser = SumType.makeDeserializer(ty.value);
    const writer = new BinaryWriter(1);
    writer.writeByte(42);
    const reader = new BinaryReader(writer.getBuffer());
    expect(() => deser(reader)).toThrow(TypeError);
  });

  test('mixed sum: unit variants → bare PascalCase string', () => {
    // A ReScript-style status enum where most variants are unit but one
    // carries a payload. The unit variants MUST come back as bare
    // capitalised strings, not {tag, value} wrappers.
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'idle', algebraicType: emptyProduct },
        { name: 'running', algebraicType: emptyProduct },
        {
          name: 'failed',
          algebraicType: AlgebraicType.Product({
            elements: [{ name: 'reason', algebraicType: AlgebraicType.String }],
          }),
        },
      ],
    });

    // idle → "Idle"
    expect(
      roundTrip(ty, w => {
        w.writeByte(0);
      })
    ).toBe('Idle');

    // running → "Running"
    expect(
      roundTrip(ty, w => {
        w.writeByte(1);
      })
    ).toBe('Running');
  });

  test('mixed sum: payloaded variant → { tag, value, _0 }', () => {
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'idle', algebraicType: emptyProduct },
        {
          name: 'failed',
          algebraicType: AlgebraicType.Product({
            elements: [{ name: 'reason', algebraicType: AlgebraicType.String }],
          }),
        },
      ],
    });

    const result = roundTrip(ty, w => {
      w.writeByte(1); // payload variant tag
      w.writeString('boom');
    });

    // ReScript @tag("tag") consumers pattern-match on `.tag` and read `._0`.
    // Legacy TS consumers still read `.value` — both must be present.
    expect(result).toMatchObject({ tag: 'Failed' });
    expect((result as any)._0).toEqual({ reason: 'boom' });
    expect((result as any).value).toEqual({ reason: 'boom' });
  });

  test('mixed sum: unknown tag throws descriptive TypeError', () => {
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'idle', algebraicType: emptyProduct },
        {
          name: 'failed',
          algebraicType: AlgebraicType.Product({
            elements: [{ name: 'reason', algebraicType: AlgebraicType.String }],
          }),
        },
      ],
    });
    const deser = SumType.makeDeserializer(ty.value);
    const writer = new BinaryWriter(1);
    writer.writeByte(99);
    const reader = new BinaryReader(writer.getBuffer());
    expect(() => deser(reader)).toThrowError(/sum type/i);
  });

  test('option sum: some(T) returns T directly; none returns undefined', () => {
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'some', algebraicType: AlgebraicType.I32 },
        { name: 'none', algebraicType: emptyProduct },
      ],
    });
    const deser = SumType.makeDeserializer(ty.value);

    const some = (() => {
      const w = new BinaryWriter(8);
      w.writeByte(0);
      w.writeI32(7);
      return deser(new BinaryReader(w.getBuffer()));
    })();
    expect(some).toBe(7);

    const none = (() => {
      const w = new BinaryWriter(1);
      w.writeByte(1);
      return deser(new BinaryReader(w.getBuffer()));
    })();
    expect(none).toBeUndefined();
  });

  test('result sum: returns {ok: T} / {err: E}', () => {
    const ty = AlgebraicType.Sum({
      variants: [
        { name: 'ok', algebraicType: AlgebraicType.I32 },
        { name: 'err', algebraicType: AlgebraicType.String },
      ],
    });
    const deser = SumType.makeDeserializer(ty.value);

    const ok = (() => {
      const w = new BinaryWriter(8);
      w.writeByte(0);
      w.writeI32(42);
      return deser(new BinaryReader(w.getBuffer()));
    })();
    expect(ok).toEqual({ ok: 42 });

    const err = (() => {
      const w = new BinaryWriter(16);
      w.writeByte(1);
      w.writeString('nope');
      return deser(new BinaryReader(w.getBuffer()));
    })();
    expect(err).toEqual({ err: 'nope' });
  });
});
