{{self.header}}

// Opaque SDK types — hold JS class instances from the SpacetimeDB SDK




// DB record — @as maps camelCase fields to snake_case runtime keys
type db = {
%% for f in &self.db_fields {
{{f}}
%% }
}

// DB and reducers access from connection
@get external db: StdbSdk.connection => db = "db"
@get external reducers: StdbSdk.connection => StdbSdk.reducers = "reducers"