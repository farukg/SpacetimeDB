%% if self.has_args {
  let {{self.name_camel}} = async (args: {{self.module}}.args) => {
%% } else {
  let {{self.name_camel}} = async () => {
%% }
    let conn = await C.getConnection()
%% if self.has_args {
    await conn->Sdk.getReducers->{{self.module}}.call_(args)
%% } else {
    await conn->Sdk.getReducers->{{self.module}}.call_
%% }
  }
