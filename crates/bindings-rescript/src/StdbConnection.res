// ReScript SpacetimeDB connection API
// This will replace the JS-based DbConnectionBuilder wrapper.

type config = {
  host: string,
  nameOrAddress: string,
  token?: string,
}

type t = {
  config: config,
  // placeholder for ws connection, etc
}

// TODO: Implement proper WS client in ReScript or thin bindings
let connect = (config: config) => {
  {config}
}
