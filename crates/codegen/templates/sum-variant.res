%% if self.payload.is_empty() {
| {{self.constructor}}
%% } else {
| {{self.constructor}}({{self.payload}})
%% }