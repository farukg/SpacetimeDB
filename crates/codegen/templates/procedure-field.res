%% if self.has_args {
  @as("{{self.accessor}}") {{self.camel}}: {{self.params_type}} => Fx.call<{{self.response_type}}>,
%% } else {
  @as("{{self.accessor}}") {{self.camel}}: unit => Fx.call<{{self.response_type}}>,
%% }
