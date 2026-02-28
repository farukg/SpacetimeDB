// StdbTransport.resi - typed interface for StdbTransport.mjs runtime helpers

// Generic row normalization: transforms BigInt→Number, Timestamp→Number, unit enums→strings.
// Shape is preserved, so caller's row type inference works.
let normalizeRow: 'a => 'a

// Encode any ReScript value to BSATN-JSON-safe representation.
// Output is suitable for JSON.stringify.
let encodeStdbValue: 'a => JSON.t

// Encode an identity hex string to its byte array representation.
let encodeIdentityHex: string => array<string>
