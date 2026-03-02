{{self.header}}
{{self.sibling_opens}}
open Types
open Sdk

// ── SDK type helpers ─────────────────────────────────────────────────

let identity = identityToHex
let connectionId = connectionIdToHex
let timestamp = timestampToFloatMs
let amount = BigInt.toFloat

// ── Newtype unwrappers ───────────────────────────────────────────────

{{self.unwrappers}}
// ── Enum toString ────────────────────────────────────────────────────

{{self.enum_to_strings}}
// ── Enum fromString ─────────────────────────────────────────────────

{{self.enum_from_strings}}
