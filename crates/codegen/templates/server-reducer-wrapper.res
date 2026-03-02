%% if self.has_args {
  let {{self.name_camel}} = async (args: {{self.module}}.args) => {
%% } else {
  let {{self.name_camel}} = async () => {
%% }
    let conn = await C.getConnection()
%% if self.has_args {
    await conn->Client.reducers->{{self.module}}.{{self.name_camel}}(args)
%% } else {
    await conn->Client.reducers->{{self.module}}.{{self.name_camel}}
%% }
  }