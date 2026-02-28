    (
      "{{self.col_name}}",
      {
        columnMetadata: {
%% if self.is_primary_key {
          isPrimaryKey: true,
%% }
        },
        typeBuilder: {algebraicType: {{self.alg_type_expr}}},
      },
    ),
