{{self.header}}

// Typed reducer/procedure records — compile-time safety layer.
// If a reducer or procedure is removed from the server module and codegen
// is re-run, any call site referencing the missing field fails to compile.
//
// This file is a leaf dependency: reducer/procedure modules never import it.
// Consumer code uses `Api.reducers` / `Api.procedures` for typed access.

type reducers = {
%% for f in &self.reducer_fields {
{{f}}
%% }
}

type procedures = {
%% for f in &self.procedure_fields {
{{f}}
%% }
}

@get external reducers: {{self.sdk_module}}.connection<{{self.sdk_module}}.remoteModule> => reducers = "reducers"
@get external procedures: {{self.sdk_module}}.connection<{{self.sdk_module}}.remoteModule> => procedures = "procedures"
