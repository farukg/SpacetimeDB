
// ── Display projection ────────────────────────────────────────────────────────
type display = {
%% for field in &self.type_fields {
{{field}}
%% }
}

let toDisplay = (row: t): display => {
%% for field in &self.body_fields {
{{field}}
%% }
}
