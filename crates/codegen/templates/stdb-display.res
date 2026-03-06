{{self.header}}
{{self.sibling_opens}}
open Sdk

// ── SDK type helpers ─────────────────────────────────────────────────

let identity = identityToHex
let connectionId = connectionIdToHex
let timestamp = timestampToFloatMs
let amount = BigInt.toFloat

// ── SDK constructors (for phantom rows / test data) ──────────────────

let emptyIdentity: identity = Identity.fromString(String.repeat("0", 64))

@new @module("spacetimedb")
external timestampFromMicros: bigint => Sdk.timestamp = "Timestamp"

let timestampFromMs = (ms: float): Sdk.timestamp =>
  timestampFromMicros(BigInt.fromInt(Float.toInt(ms)) * 1000n)

// ── Newtype unwrappers ───────────────────────────────────────────────

{{self.unwrappers}}
// ── Enum toString ────────────────────────────────────────────────────

{{self.enum_to_strings}}
// ── Enum fromString ─────────────────────────────────────────────────

{{self.enum_from_strings}}
// ── Sum enum toString ───────────────────────────────────────────────

{{self.sum_to_strings}}
