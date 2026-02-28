// PK index
type {{self.field_camel}}Index
@get external {{self.field_camel}}: handle => {{self.field_camel}}Index = "{{self.field_raw}}"
@send external find: ({{self.field_camel}}Index, {{self.find_param_type}}) => Nullable.t<t> = "find"