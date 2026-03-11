type cleanup = unit => unit
type callbackIssue
type foreignCall<'a>
type foreignFailure = {message: string}
type timeoutIssue = {milliseconds: int}

type issue =
  | ForeignFailure(foreignFailure)
  | TimedOut(timeoutIssue)

type call<'a> = (~onValue: 'a => unit, ~onError: issue => unit) => cleanup

external toNullableObj: 'a => Nullable.t<Obj.t> = "%identity"
@get external messageField: Obj.t => Nullable.t<string> = "message"

@send external attachValue: (foreignCall<'a>, 'a => unit) => foreignCall<'a> = "then"
@send external attachIssue: (foreignCall<'a>, callbackIssue => unit) => foreignCall<'a> = "catch"

let noop = () => ()

let issueText = issue =>
  switch issue {
  | ForeignFailure({message}) => message
  | TimedOut({milliseconds}) =>
    `Timed out waiting for SpacetimeDB after ${milliseconds->Int.toString}ms`
  }

let fromCallbackIssue = raw =>
  switch raw->toNullableObj->Nullable.toOption {
  | Some(rawObj) =>
    switch rawObj->messageField->Nullable.toOption {
    | Some(message) if message !== "" => ForeignFailure({message: message})
    | _ => ForeignFailure({message: "SpacetimeDB call failed"})
    }
  | None => ForeignFailure({message: "SpacetimeDB call failed"})
  }

let pure = (value: 'a): call<'a> =>
  (~onValue, ~onError as _) => {
    onValue(value)
    noop
  }

let observe = (call, ~onValue, ~onError) =>
  call(~onValue, ~onError)

let map = (call, project): call<'b> =>
  (~onValue, ~onError) =>
    call(
      ~onValue=value => onValue(project(value)),
      ~onError,
    )

let flatMap = (call, bind): call<'b> =>
  (~onValue, ~onError) => {
    let nestedCleanup = ref(noop)
    let outerCleanup =
      call(
        ~onValue=value => nestedCleanup.contents = bind(value)(~onValue, ~onError),
        ~onError,
      )

    () => {
      outerCleanup()
      nestedCleanup.contents()
    }
  }

let capture = (call: call<'a>): call<result<'a, issue>> =>
  (~onValue, ~onError as _) =>
    call(
      ~onValue=value => onValue(Ok(value)),
      ~onError=callIssue => onValue(Error(callIssue)),
    )

let fromThunk = makeCall => makeCall()

let fromForeignCall = (foreign: foreignCall<'a>): call<'a> =>
  (~onValue, ~onError) => {
    let active = ref(true)
    foreign->attachValue(value => {
      if active.contents {
        onValue(value)
      }
    })->ignore
    foreign->attachIssue(raw => {
      if active.contents {
        onError(fromCallbackIssue(raw))
      }
    })->ignore
    () => active := false
  }

let run = call => observe(call, ~onValue=_ => (), ~onError=_ => ())->ignore
