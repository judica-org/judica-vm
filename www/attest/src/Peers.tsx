import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Cancel, Start } from '@mui/icons-material';
import { AddPeer } from './AddPeer';

function CustomToolbar(root: string) {
  return ()=>(
    <GridToolbarContainer>
      <h4>Peers</h4>
      <AddPeer root={root}></AddPeer>
    </GridToolbarContainer>
  );
}
export function Peers(props: { peers: Array<{ service_url: string; port: string; fetch_from: boolean; push_to: boolean; }>, root: string }) {

  const rows: GridRowsProp = props.peers.map((peer, id) => {
    const row: typeof rows[number] = Object.fromEntries(Object.entries(peer));
    row.id = id;
    return row;
  });
  const columns = [
    { field: 'service_url', headerName: 'Host', minWidth: 100 },
    { field: 'port', headerName: 'Port', minWidth: 100 },
    { field: 'fetch_from', headerName: 'Fetch', minWidth: 50 },
    {
      headerName: 'Fetch Actions',
      field: 'fetch_pactions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Cancel></Cancel>} onClick={() => console.log(["Peer"], "TODO")} label="Log Message" />,
        <GridActionsCellItem icon={<Start></Start>} onClick={() => console.log(["Peer"], "TODO")} label="Log Message" />
      ]
    },
    { field: 'push_to', headerName: 'Push', width: 50 },
    {
      headerName: 'Push Actions',
      field: 'push_actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Cancel></Cancel>} onClick={() => console.log(["Peer"], "TODO")} label="Log Message" />,
        <GridActionsCellItem icon={<Start></Start>} onClick={() => console.log(["Peer"], "TODO")} label="Log Message" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} components={{ Toolbar: CustomToolbar(props.root) }} />;
}
