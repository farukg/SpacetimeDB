{{self.header}}

{{self.args_record}}
@send external {{self.accessor}}: (StdbTypes.reducers, args) => promise<unit> = "{{self.accessor}}"

let call = (conn: StdbTypes.connection, {{self.call_params}}) =>
  conn->StdbClient.reducers->{{self.accessor}}({
{{self.call_body_fields}}
  })

{{self.react_hooks}}