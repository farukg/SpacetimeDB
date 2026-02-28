%% if self.has_args {
let {{self.name_camel}} = async (args: {{self.module}}.args) => {
%% } else {
let {{self.name_camel}} = async () => {
%% }
  let conn: StdbTypes.connection = Obj.magic(await StdbServerConnection.getConnection())
%% if self.has_args {
  let result = await conn->StdbClient.reducers->{{self.module}}.{{self.name_camel}}(args)
%% } else {
  let result = await conn->StdbClient.reducers->{{self.module}}.{{self.name_camel}}
%% }
  if syncDelayMs > 0 { await sleep(syncDelayMs) }
  result
}