{{self.header}}

// Opaque SDK types — hold JS class instances from the SpacetimeDB SDK




// DB record — @as maps camelCase fields to snake_case runtime keys
type db = {
%% for f in &self.db_fields {
{{f}}
%% }
}

// DB, reducers, and procedures access from connection
@get external db: {{self.sdk_module}}.connection => db = "db"
@get external reducers: {{self.sdk_module}}.connection => {{self.sdk_module}}.reducers = "reducers"
@get external procedures: {{self.sdk_module}}.connection => {{self.sdk_module}}.procedures = "procedures"