// GENERATED — DO NOT EDIT
// Default observer runtime backed by CallSupport.

module Fx = {{self.root_module}}__Fx

type bridgeCall<'a> = Stdb__CallSupport.call<'a>
type bridgeIssue = Stdb__CallSupport.issue

external toBridgeCall: Fx.call<'a> => bridgeCall<'a> = "%identity"
external fromBridgeCall: bridgeCall<'a> => Fx.call<'a> = "%identity"
external fromBridgeIssue: bridgeIssue => Fx.error = "%identity"
external toBridgeIssue: Fx.error => bridgeIssue = "%identity"

type effect<'a> = Fx.call<'a>

let observe = (call, ~onValue, ~onError) =>
  Stdb__CallSupport.observe(
    toBridgeCall(call),
    ~onValue,
    ~onError=issue => onError(issue->fromBridgeIssue),
  )

let map = (call, project) =>
  call->toBridgeCall->(inner => Stdb__CallSupport.map(inner, project))->fromBridgeCall

let flatMap = (call, bind) =>
  call
  ->toBridgeCall
  ->(inner =>
    Stdb__CallSupport.flatMap(inner, value => bind(value)->toBridgeCall)
  )
  ->fromBridgeCall

let capture = call =>
  call
  ->toBridgeCall
  ->Stdb__CallSupport.capture
  ->(inner =>
    Stdb__CallSupport.map(inner, result =>
      switch result {
      | Ok(value) => Ok(value)
      | Error(issue) => Error(issue->fromBridgeIssue)
      }
    )
  )
  ->fromBridgeCall

let describeError = error =>
  error->toBridgeIssue->Stdb__CallSupport.issueText

let pure = value =>
  Stdb__CallSupport.pure(value)->fromBridgeCall

let fromCall = makeCall =>
  Stdb__CallSupport.fromThunk(() => makeCall()->toBridgeCall)->fromBridgeCall

let run = call => call->observe(~onValue=_ => (), ~onError=_ => ())->ignore
