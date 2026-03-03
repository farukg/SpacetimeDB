let {{self.config_name}}: Hooks.tableConfig<{{self.table_module}}.t> =
  Hooks.mkTable(conn => { let d: Client.db = conn->Client.db; d.{{self.accessor}} }, {{self.table_module}}.iter)
