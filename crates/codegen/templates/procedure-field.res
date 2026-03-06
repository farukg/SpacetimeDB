%% if self.has_args {
  @as("{{self.accessor}}") {{self.camel}}: {{self.params_type}} => promise<{{self.response_type}}>,
%% } else {
  @as("{{self.accessor}}") {{self.camel}}: unit => promise<{{self.response_type}}>,
%% }
