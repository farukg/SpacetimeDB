{{self.header}}

// Opaque SDK types — hold JS class instances from the SpacetimeDB SDK




// DB record — @as maps camelCase fields to snake_case runtime keys
type db = {
%% for f in &self.db_fields {
{{f}}
%% }
}

// DB and reducers access from connection
@get external db: StdbTypes.connection => db = "db"
@get external reducers: StdbTypes.connection => StdbTypes.reducers = "reducers"