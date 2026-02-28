{{self.header}}

@send external {{self.accessor}}: StdbSdk.reducers => promise<unit> = "{{self.accessor}}"

let call = (conn: StdbSdk.connection) =>
  conn->StdbClient.reducers->{{self.accessor}}

{{self.react_hooks}}