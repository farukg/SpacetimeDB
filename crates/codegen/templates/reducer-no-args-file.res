{{self.header}}

@send external {{self.accessor}}: StdbTypes.reducers => promise<unit> = "{{self.accessor}}"

let call = (conn: StdbTypes.connection) =>
  conn->StdbClient.reducers->{{self.accessor}}

{{self.react_hooks}}