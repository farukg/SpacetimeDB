// Opaque SDK types
type connection
type eventCtx
type reducers

// Opaque SDK Timestamp — use toDate, toMillis, or toFloatMs
type timestamp
@send external toMillis: (timestamp) => bigint = "toMillis"
@send external toDate: (timestamp) => Date.t = "toDate"
let toFloatMs = (ts: timestamp): float => ts->toMillis->BigInt.toFloat

type timeDuration  // opaque — SDK TimeDuration class instance
@send external timeDurationToMicros: (timeDuration) => bigint = "toMicros"

// ScheduleAt — SDK built-in tagged union
@tag("tag")
type scheduleAt =
  | Interval({value: timeDuration})
  | Time({value: timestamp})

