// GENERATED — DO NOT EDIT
// Module type contracts for observer/functor mode.
// Implement these once in your app; all reducer and table functors accept them.

// EFFECT_RUNTIME — for reducer calls.
// E.effect<'a> is your effect/stream type.
// fromPromise converts the SDK's promise boundary into your effect type (thunked).
// toPromise is the inverse — for interop with promise-based APIs.
// run executes as fire-and-forget without exposing promise.
module type EFFECT_RUNTIME = {
  type effect<'a>
  let fromPromise: (unit => promise<'a>) => effect<'a>
  let toPromise: effect<'a> => promise<'a>
  let run: effect<'a> => unit
}

// OBSERVER — for table subscriptions.
// O.stream<'a> is your stream type.
// fromCallback receives: emit => cleanup. Cleanup is strict (always callable, may be no-op).
// merge combines N streams of same type.
module type OBSERVER = {
  type stream<'a>
  let fromCallback: (('a => unit) => unit => unit) => stream<'a>
  let merge: array<stream<'a>> => stream<'a>
}
