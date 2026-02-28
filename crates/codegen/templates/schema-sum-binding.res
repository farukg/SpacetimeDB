let {{self.binding_name}} = AlgType.sum([
%% for v in &self.variants {
{{v}}
%% }
])
