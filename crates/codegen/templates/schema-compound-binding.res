let {{self.binding_name}} = Compound(
%% if self.is_sum {
  Sum({value: {variants: [
%% } else {
  Product({value: {elements: [
%% }
%% for item in &self.items {
{{item}}
%% }
]}})
)
