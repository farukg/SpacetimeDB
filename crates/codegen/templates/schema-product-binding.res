let {{self.binding_name}} = AlgType.product([
%% for elem in &self.elements {
{{elem}}
%% }
])
