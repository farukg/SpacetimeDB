%% if self.has_args {
    {{self.name_camel}}: {{self.module}}.args => promise<unit>,
%% } else {
    {{self.name_camel}}: unit => promise<unit>,
%% }