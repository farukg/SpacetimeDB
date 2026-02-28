// Typed bindings for StdbTransport.mjs runtime helpers.
// The .mjs file stays as JavaScript (runtime type dispatch via typeof/in).
// These externals provide typed access from ReScript.

// Generic row normalization: transforms BigInt→Number, Timestamp→Number, unit enums→strings.
// Shape is preserved, so caller's row type inference works.
@module("./StdbTransport.mjs")
external normalizeRow: 'a => 'a = "normalizeRow"

// Encode any ReScript value to BSATN-JSON-safe representation.
// Output is suitable for JSON.stringify.
@module("./StdbTransport.mjs")
external encodeStdbValue: 'a => JSON.t = "encodeStdbValue"

// Encode an identity hex string to its byte array representation.
@module("./StdbTransport.mjs")
external encodeIdentityHex: string => array<string> = "encodeIdentityHex"
