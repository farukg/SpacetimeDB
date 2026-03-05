    {
      name: "{{self.procedure_name}}",
      accessorName: "{{self.accessor_name}}",
      params: {
        elements: [
%% for elem in &self.param_elements {
{{elem}}
%% }
        ],
      },
      returnType: {algebraicType: {{self.return_type_expr}}},
    },
