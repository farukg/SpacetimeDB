%% if self.has_args {
  @as("{{self.accessor}}") {{self.camel}}: {{self.args_type}} => Fx.call<unit>,
%% } else {
  @as("{{self.accessor}}") {{self.camel}}: unit => Fx.call<unit>,
%% }
