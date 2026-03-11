// GENERATED — DO NOT EDIT
// Hook-oriented call contracts. Concrete transport execution is caller-provided.

type call<'a>
type error
type cleanup = unit => unit

module type CALL_RUNTIME = {
  type effect<'a>
  let observe: (call<'a>, ~onValue: 'a => unit, ~onError: error => unit) => cleanup
  let map: (call<'a>, 'a => 'b) => call<'b>
  let flatMap: (call<'a>, 'a => call<'b>) => call<'b>
  let capture: call<'a> => call<result<'a, error>>
  let describeError: error => string
  let fromCall: (unit => call<'a>) => effect<'a>
  let run: effect<'a> => unit
}
