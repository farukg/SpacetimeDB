%% if self.has_args {
  @as("{{self.accessor}}") {{self.camel}}: {{self.args_type}} => promise<unit>,
%% } else {
  @as("{{self.accessor}}") {{self.camel}}: unit => promise<unit>,
%% }
