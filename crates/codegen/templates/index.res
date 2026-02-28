{{self.header}}

module StdbTypes = StdbTypes
module StdbClient = StdbClient

module Tables = {
%% for a in &self.table_aliases {
  {{a}}
%% }
}

module Reducers = {
%% for a in &self.reducer_aliases {
  {{a}}
%% }
}

module Procedures = {
%% for a in &self.procedure_aliases {
  {{a}}
%% }
}