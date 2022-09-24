import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Cancel, Start } from '@mui/icons-material';
import { AddPeer } from './AddPeer';

function CustomToolbar(peer: any) {
  return () => {
    return (<GridToolbarContainer>
      < h4 > Peers</h4 >
      {peer}
    </GridToolbarContainer >);
  };
}

function change(url: string, port: number, root: string,
  { push_to,
    fetch_from,
    allow_unsolicited_tips }: {
      push_to?: boolean,
      fetch_from?: boolean,
      allow_unsolicited_tips?: boolean
    }) {

  fetch(`${root}/service`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        url, port, push_to, fetch_from, allow_unsolicited_tips
      })
    });
}
type Peer = {
  service_url: string,
  port: string,
  fetch_from: boolean,
  push_to: boolean,
  allow_unsolicited_tips: boolean
};
export function Peers(props: { peers: Array<Peer>, root: string | null, toolbar_component: any }) {

  const root = props.root;
  const components = { Toolbar: CustomToolbar(props.toolbar_component) };
  const rows: GridRowsProp = props.peers.map((peer, id) => {
    const row: typeof rows[number] = Object.fromEntries(Object.entries(peer));
    row.id = id;
    return row;
  });

  const fetch_actions = root === null ? {
    headerName: 'Fetch Actions',
    field: 'fetch_pactions',
    type: 'actions',
    hide: true,
  } : {
    headerName: 'Fetch Actions',
    field: 'fetch_pactions',
    type: 'actions',
    getActions: (params: GridRowParams) => [
      <GridActionsCellItem icon={<Cancel></Cancel>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { fetch_from: false });
        }}
        label="Fetch Enable" />,
      <GridActionsCellItem icon={<Start></Start>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { fetch_from: true });
        }}
        label="Fetch Disable" />
    ]
  };
  const push_actions = root === null ? {

    headerName: 'Push Actions',
    field: 'push_actions',
    type: 'actions',
    hide: true
  } : {
    headerName: 'Push Actions',
    field: 'push_actions',
    type: 'actions',
    getActions: (params: GridRowParams) => [
      <GridActionsCellItem icon={<Cancel></Cancel>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { push_to: false });
        }}
        label="Push Enable" />,
      <GridActionsCellItem icon={<Start></Start>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { push_to: true });
        }}
        label="Push Disable" />
    ]
  };
  const new_tip_actions = root === null ? {
    headerName: 'New Tip Actions',
    field: 'new_tip_actions',
    type: 'actions',
    hide: true,
  } : {
    headerName: 'New Tip Actions',
    field: 'new_tip_actions',
    type: 'actions',
    getActions: (params: GridRowParams) => [
      <GridActionsCellItem icon={<Cancel></Cancel>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { allow_unsolicited_tips: false });
        }}
        label="Push Enable" />,
      <GridActionsCellItem icon={<Start></Start>}
        onClick={() => {
          const row = rows[params.id as number];
          change(row.service_url, row.port, root, { allow_unsolicited_tips: true });
        }}
        label="Push Disable" />
    ]
  };
  const columns = [
    { field: 'service_url', headerName: 'Host', minWidth: 100 },
    { field: 'port', headerName: 'Port', minWidth: 100 },
    { field: 'fetch_from', headerName: 'Fetch', minWidth: 25 },
    fetch_actions,
    { field: 'push_to', headerName: 'Push', width: 25 },
    push_actions,
    { field: 'allow_unsolicited_tips', headerName: 'New Tips', minWidth: 25 },
    new_tip_actions,
  ];

  return <DataGrid rows={rows} columns={columns} components={components} />;
}
