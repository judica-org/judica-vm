import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Report } from '@mui/icons-material';
import { Button } from '@mui/material';
import { Envelope } from './App';

export function ExpensiveMsgDB(props: { url: string; }) {
  const [data, set_data] = React.useState<Record<string, Envelope>>({});
  const [text, set_text] = React.useState("");
  const handle = async () => {
    const target = `${props.url}/expensive_db_snapshot`;
    console.log("Fetching...", target);
    try {
      const resp = await fetch(target);
      const js = await resp.json();
      set_data(js);
    }
    catch { }
  };
  function CustomToolbar() {
    return (
      <GridToolbarContainer>
        <h4>All Messages in DB</h4>
        <Button onClick={handle}> Refresh </Button>
      </GridToolbarContainer>
    );
  }

  const rows: GridRowsProp = Object.entries(data).map(([msg_hash, envelope]: [string, Envelope], id) => {
    const genesis = envelope.header.ancestors?.genesis ?? msg_hash;
    const msg = JSON.stringify(envelope.msg);
    const signing_key = envelope.header.key;
    return { id, genesis, msg, signing_key, hash: msg_hash, height: envelope.header.height };
  });
  const columns = [
    { field: 'genesis', headerName: 'Genesis', width: 300 },
    { field: 'signing_key', headerName: 'Public Key', width: 300 },
    { field: 'hash', headerName: 'Message Hash', width: 150 },
    { field: 'height', headerName: 'Height', width: 150 },
    { field: 'msg', headerName: 'Message', width: 150 },
    {
      headerName: 'Log Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Report></Report>} onClick={() => console.log(["message"], JSON.parse(rows[params.id as number].msg))} label="Log Message" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} components={{ Toolbar: CustomToolbar }} />;
}
