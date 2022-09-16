import React from 'react';
import { DataGrid, GridRowsProp } from '@mui/x-data-grid';

export function TaskSet(props: { tasks: Array<[string, number, "Fetch" | "Push"]>; }) {
  const rows: GridRowsProp = props.tasks.map(([host, port, typ], id) => {
    return { id, host, port, typ };
  });
  const columns = [
    { field: 'host', headerName: 'Host', minWidth: 100 },
    { field: 'port', headerName: 'Port', minWidth: 100 },
    { field: 'typ', headerName: 'Type', minWidth: 50 },
  ];

  return <DataGrid rows={rows} columns={columns} />;
}
