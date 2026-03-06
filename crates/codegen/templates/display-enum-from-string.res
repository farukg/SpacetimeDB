let {{self.fn_name}} = (v: string): option<Types.{{self.module_name}}.t> =>
  switch v {
%% for arm in &self.arms {
{{arm}}
%% }
  | _ => None
  }
