    {
      name: "{{self.reducer_name}}",
      accessorName: "{{self.accessor_name}}",
      paramsType: {
        elements: [
%% for elem in &self.param_elements {
{{elem}}
%% }
        ],
      },
    },
