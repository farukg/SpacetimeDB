// GENERATED — DO NOT EDIT
// Event-based file-to-bytes helper.

type readIssue =
  | ReadFailed(string)
  | ReadAborted

module Uint8Array = {
  type t
  @new @scope("globalThis") external fromBuffer: ArrayBuffer.t => t = "Uint8Array"
}

module FileReader = {
  type t
  @new @scope("globalThis")
  external make: unit => t = "FileReader"
  @set external setOnload: (t, {..} => unit) => unit = "onload"
  @set external setOnerror: (t, {..} => unit) => unit = "onerror"
  @set external setOnabort: (t, {..} => unit) => unit = "onabort"
  @get @return(nullable) external resultArrayBuffer: t => option<ArrayBuffer.t> = "result"
  @send external readAsArrayBuffer: (t, 'file) => unit = "readAsArrayBuffer"
}

@val @scope("Array") external toIntArray: Uint8Array.t => array<int> = "from"

let issueText = issue =>
  switch issue {
  | ReadFailed(message) => message
  | ReadAborted => "File read aborted"
  }

let read = (~file: 'file, ~onSuccess: array<int> => unit, ~onError: readIssue => unit) => {
  let reader = FileReader.make()

  reader->FileReader.setOnload(_ => {
    switch reader->FileReader.resultArrayBuffer {
    | Some(buffer) => onSuccess(buffer->Uint8Array.fromBuffer->toIntArray)
    | None => onError(ReadFailed("Unable to read file"))
    }
  })
  reader->FileReader.setOnerror(_ => onError(ReadFailed("Unable to read file")))
  reader->FileReader.setOnabort(_ => onError(ReadAborted))
  reader->FileReader.readAsArrayBuffer(file)
}
