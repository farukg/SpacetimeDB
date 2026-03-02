let {{self.binding_name}} = Compound(Product({value: {elements: [
%% for elem in &self.elements {
{{elem}}
%% }
]}}))
