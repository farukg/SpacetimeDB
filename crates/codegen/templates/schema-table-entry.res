    (
      "{{self.accessor_name}}",
      {
        sourceName: "{{self.source_name}}",
        accessorName: "{{self.accessor_name}}",
        rowType: {
          elements: [
%% for elem in &self.row_elements {
{{elem}}
%% }
          ],
        },
        columns: Dict.fromArray([
%% for col in &self.columns {
{{col}}
%% }
        ]),
        indexes: [
%% for idx in &self.indexes {
{{idx}}
%% }
        ],
        constraints: [
%% for c in &self.constraints {
{{c}}
%% }
        ],
%% if self.is_event {
        isEvent: true,
%% }
      },
    ),
