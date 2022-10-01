import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Cancel, CheckBox, CheckBoxOutlineBlank, ContentCopy, Pending, Start, ToggleOffTwoTone, ToggleOnTwoTone, WindowOutlined } from '@mui/icons-material';
import { AddPeer } from './AddPeer';
import { PeerInfo } from './App';

function CustomToolbar(peer: any) {
  return () => {
    return (<GridToolbarContainer>
      < h4 > Peers</h4 >
      {peer}
    </GridToolbarContainer >);
  };
}

async function change(url: string, port: number, root: string,
  { push_to,
    fetch_from,
    allow_unsolicited_tips }: {
      push_to?: boolean,
      fetch_from?: boolean,
      allow_unsolicited_tips?: boolean
    }) {

  await fetch(`${root}/service`,
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
export function Peers(props: { peers: Array<PeerInfo>, root: string | null, toolbar_component: any }) {

  const root = props.root;
  const components = { Toolbar: CustomToolbar(props.toolbar_component) };
  const rows: GridRowsProp = props.peers.map((peer, id) => {
    const row: typeof rows[number] = Object.fromEntries(Object.entries(peer));
    row.id = id;
    row.service = `${row.service_url}:${row.port}`;
    row.pending_change_fetch = false;
    row.pending_change_push = false;
    row.pending_change_unsolicit = false;
    return row;
  });

  const fetch_actions = root === null ? {
    headerName: 'Fetch Actions',
    field: 'fetch_actions',
    type: 'actions',
    hide: true,
  } : {
    headerName: 'Fetch Actions',
    field: 'fetch_actions',
    type: 'actions',
    minWidth: 150,
    getActions: (params: GridRowParams) => {
      const row = rows[params.id as number];
      if (row.pending_change_fetch)
        return [<GridActionsCellItem icon={<Pending></Pending>} onClick={() => { }} label="pending"></GridActionsCellItem>];
      if (row.fetch_from)
        return [
          <GridActionsCellItem icon={<CheckBox></CheckBox>}
            onClick={async () => {
              const row = rows[params.id as number];
              row.pending_change_fetch = true;
              // speculative
              row.fetch_from = false;
              await change(row.service_url, row.port, root, { fetch_from: false });
              row.pending_change_fetch = false;
            }}
            label="Uncheck to Disable Fetch" />,]
      else
        return [
          <GridActionsCellItem icon={<CheckBoxOutlineBlank></CheckBoxOutlineBlank>}
            onClick={async () => {
              const row = rows[params.id as number];
              row.pending_change_fetch = true;
              // speculative
              row.fetch_from = true;
              await change(row.service_url, row.port, root, { fetch_from: true });
              row.pending_change_fetch = false;
            }}
            label="Check to Enable Fetch" />
        ];
    }
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
    minWidth: 150,
    getActions: (params: GridRowParams) => {
      const row = rows[params.id as number];
      if (row.pending_change_push)
        return [<GridActionsCellItem icon={<Pending></Pending>} onClick={() => { }} label="pending"></GridActionsCellItem>];
      if (row.push_to)
        return [
          <GridActionsCellItem icon={<CheckBox></CheckBox>}
            onClick={async () => {
              const row = rows[params.id as number];
              row.pending_change_push = true;
              // speculative
              row.push_to = false;
              await change(row.service_url, row.port, root, { push_to: false });
              row.pending_change_push = false;
            }}
            label="Uncheck to Disable Push" />,]
      else
        return [<GridActionsCellItem icon={<CheckBoxOutlineBlank></CheckBoxOutlineBlank>}
          onClick={async () => {
            const row = rows[params.id as number];
            row.pending_change_push = true;
            // speculative
            row.push_to = true;
            await change(row.service_url, row.port, root, { push_to: true });
            row.pending_change_push = false;
          }}
          label="Check to Enable Push" />
        ];
    }
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
    minWidth: 150,
    getActions: (params: GridRowParams) => {

      const row = rows[params.id as number];
      if (row.pending_change_unsolicit)
        return [<GridActionsCellItem icon={<Pending></Pending>} onClick={() => { }} label="pending"></GridActionsCellItem>];
      if (row.allow_unsolicited_tips)
        return [
          <GridActionsCellItem icon={<CheckBox></CheckBox>}
            onClick={async () => {
              row.pending_change_unsolicit = true;
              // speculative
              row.allow_unsolicited_tips = false;
              await change(row.service_url, row.port, root, { allow_unsolicited_tips: false });
              row.pending_change_unsolicit = false;
            }}
            label="Uncheck to Disable New Tips" />];
      else
        return [
          <GridActionsCellItem icon={<CheckBoxOutlineBlank></CheckBoxOutlineBlank>}
            onClick={async () => {
              row.pending_change_unsolicit = true;
              // speculative
              row.allow_unsolicited_tips = true;
              await change(row.service_url, row.port, root, { allow_unsolicited_tips: true });
              row.pending_change_unsolicit = false;
            }}
            label="Check to Enable New Tips" />
        ]
    }
  };
  const columns = [
    { field: 'service', headerName: 'Server', minWidth: 100 },
    {
      headerName: 'Copy',
      field: 'copy_to_clipboard',
      type: 'actions',
      minWidth: 50,
      getActions: (params: GridRowParams) => {
        const row = rows[params.id as number];
        return [<GridActionsCellItem icon={<ContentCopy></ContentCopy>} onClick={() => {
          window.navigator.clipboard.writeText(row.service);
        }} label="pending"></GridActionsCellItem>];
      }
    },
    fetch_actions,
    push_actions,
    new_tip_actions,
  ];

  return <DataGrid rows={rows} columns={columns} components={components} />;
}
