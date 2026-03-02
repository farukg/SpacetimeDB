{{self.header}}
open StdbTypes
open {{self.sdk_module}}

// ── SDK type helpers ─────────────────────────────────────────────────

let identity = identityToHex
let connectionId = connectionIdToHex
let timestamp = timestampToFloatMs
let amount = BigInt.toFloat

// ── Newtype unwrappers ───────────────────────────────────────────────

{{self.unwrappers}}
// ── Enum toString ────────────────────────────────────────────────────

{{self.enum_to_strings}}
