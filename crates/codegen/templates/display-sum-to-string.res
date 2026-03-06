let {{self.fn_name}} = (v: Types.{{self.module_name}}.t) =>
  switch v {
%% for arm in &self.arms {
{{arm}}
%% }
  }
