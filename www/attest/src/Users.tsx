import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Create } from '@mui/icons-material';
import { handle_new_msg } from './App';
import { MakeGenesis } from './MakeGenesis';

export function Users(props: { users: Array<[string, string, boolean]>; url: string; }) {

  const rows: GridRowsProp = props.users.map(([pubkey, nickname, has_private_key], id) => {
    return { id, pubkey, nickname, has_private_key };
  });
  const columns = [
    { field: 'pubkey', headerName: 'Public Key', minWidth: 300 },
    { field: 'nickname', headerName: 'Nickname', minWidth: 150 },
    { field: 'has_private_key', headerName: 'Known Secret Key', minWidth: 150 },
    {
      headerName: 'New Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Create></Create>} onClick={() => handle_new_msg(props.url, rows[params.id as number].pubkey)} label="Create New Message" />,
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} components={{ Toolbar: CustomToolbar(props.url) }} />;

}
function CustomToolbar(url: string) {
  return () => (
    <GridToolbarContainer>
      <h4>Chains</h4>
      <MakeGenesis url={url}></MakeGenesis>
    </GridToolbarContainer>
  );
}
