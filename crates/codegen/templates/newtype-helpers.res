let make = (v: {{self.inner_type}}): t => { {{self.field_camel}}: v }
let value = (v: t): {{self.inner_type}} => v.{{self.field_camel}}
%% if let Some(to_key) = &self.to_key_expr {
let toKey = (v: t): string => {{to_key}}
%% }
let equal = (a: t, b: t): bool => a.{{self.field_camel}} == b.{{self.field_camel}}
