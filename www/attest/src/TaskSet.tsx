import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp } from '@mui/x-data-grid';
import { ContentCopy } from '@mui/icons-material';

export function TaskSet(props: { tasks: Array<[[string, number], "Fetch" | "Push", boolean]>; }) {
  const rows: GridRowsProp = props.tasks.map(([[host, port], typ, allow_unsolicited_tips], id) => {
    console.log(host, port);
    return { id, server: `${host}:${port}`, typ };
  });
  const columns = [
    { field: 'server', headerName: 'server', minWidth: 100 },
    {
      headerName: 'Copy',
      field: 'copy_to_clipboard',
      type: 'actions',
      minWidth: 50,
      getActions: (params: GridRowParams) => {
        const row = rows[params.id as number];
        return [<GridActionsCellItem icon={<ContentCopy></ContentCopy>} onClick={() => {
          window.navigator.clipboard.writeText(row.server);
        }} label="pending"></GridActionsCellItem>];
      }
    },
    { field: 'typ', headerName: 'Type', minWidth: 50 },
  ];

  return <DataGrid rows={rows} columns={columns} />;
}
