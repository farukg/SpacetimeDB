let {{self.fn_name}} = (v: {{self.module_name}}.t) =>
  switch v {
%% for arm in &self.arms {
{{arm}}
%% }
  }

