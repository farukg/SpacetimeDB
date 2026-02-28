// Singleton server connection manager for SpacetimeDB.
// Configure once, then getConnection() returns a lazy-connected promise.

type remoteModule
type connection

type config = {
  remoteModule: remoteModule,
  databaseName: string,
  uri: string,
  allTables?: array<string>,
  connectTimeoutMs?: int,
}

let configureServerConnection: config => unit
let getConnection: unit => promise<connection>
let resetServerConnectionForTests: unit => unit
