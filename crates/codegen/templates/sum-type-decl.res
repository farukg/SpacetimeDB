@tag("tag")
{{self.keyword}} {{self.name}} =
%% for v in &self.variants {
  {{v}}
%% }
