{{self.header}}

// Schema-specific table configs for {{self.root_module}}__Hooks.useRows().
// Each config bundles a table handle accessor + iteration function.
//
// Usage:
//   let rows = Hooks.useRows(Bridge.myReceipts)
//   module CallHooks = Hooks.CallHooks(MyCallRuntime)
//   let {call} = CallHooks.useCallWith(Reducers.SaveReceipt.call)

module Hooks = {{self.root_module}}__Hooks
module Client = {{self.root_module}}__Client

%% for entry in &self.table_entries {
{{entry}}
%% }
