{{self.header}}

{{self.args_record}}
@send external {{self.accessor}}: (StdbSdk.reducers, args) => promise<unit> = "{{self.accessor}}"

let call = (conn: StdbSdk.connection, {{self.call_params}}) =>
  conn->StdbClient.reducers->{{self.accessor}}({
{{self.call_body_fields}}
  })

{{self.react_hooks}}