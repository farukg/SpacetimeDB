{{self.header}}

// DB record — @as maps camelCase fields to snake_case runtime keys
type db = {
%% for f in &self.db_fields {
{{f}}
%% }
}

// DB access from connection
@get external db: {{self.sdk_module}}.connection => db = "db"
