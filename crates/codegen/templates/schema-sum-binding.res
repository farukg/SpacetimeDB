let {{self.binding_name}} = Compound(Sum({value: {variants: [
%% for v in &self.variants {
{{v}}
%% }
]}}))
