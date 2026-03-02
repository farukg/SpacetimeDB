{{self.header}}

// StdbSchema.res — pure ReScript runtime schema.
// Constructs remoteModule directly using {{self.sdk_module}} types.
// SpacetimeDB CLI Version: {{self.cli_version}}

open {{self.sdk_module}}

// ─── Named type algebraicType bindings ──────────────────────────────

{{self.type_bindings}}
// ─── Remote module assembly ─────────────────────────────────────────

let remoteModule: remoteModule = {
  versionInfo: {cliVersion: "{{self.cli_version}}"},
  tables: Dict.fromArray([
%% for t in &self.table_entries {
{{t}}
%% }
  ]),
  reducers: [
%% for r in &self.reducer_entries {
{{r}}
%% }
  ],
  procedures: [
%% for p in &self.procedure_entries {
{{p}}
%% }
  ],
}

let allTableNames = [
%% for name in &self.all_table_names {
  "{{name}}",
%% }
]

let tables = makeQueryBuilder({tables: remoteModule.tables})
let reducers = convertToAccessorMap(remoteModule.reducers)
