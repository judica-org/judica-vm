// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Create, Dangerous, WaterDamage } from '@mui/icons-material';
import { handle_new_msg } from './App';
import { MakeGenesis, MakeGenesisImported } from './MakeGenesis';

export function Users(props: { users: Array<[string, string, boolean, string | null]>; url: string; }) {

  const rows: GridRowsProp = props.users.map(([pubkey, nickname, has_private_key, leaked], id) => {
    return { id, pubkey, nickname, has_private_key, leaked_private_key: leaked !== null, leaked };
  });
  const columns = [
    { field: 'pubkey', headerName: 'Public Key', minWidth: 300 },
    { field: 'nickname', headerName: 'Nickname', minWidth: 150 },
    { field: 'has_private_key', headerName: 'Known Secret Key', minWidth: 150 },
    { field: 'leaked_private_key', headerName: 'Leaked Secret Key', minWidth: 150 },
    {
      headerName: 'New Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => {

        if (rows[params.id as number].has_private_key)
          return [
            <GridActionsCellItem icon={<Create></Create>} onClick={() => handle_new_msg("SAFE", props.url, rows[params.id as number].pubkey)} label="Create New Message" />,
            <GridActionsCellItem icon={<Dangerous></Dangerous>} onClick={() => handle_new_msg("DANGER", props.url, rows[params.id as number].pubkey)} label="Create New Message" />,
          ];
        else
          return []
      }
    },
    {
      headerName: 'Leaked Key',
      field: 'leaky_actions',
      type: 'actions',
      getActions: (params: GridRowParams) => {

        if (rows[params.id as number].leaked_private_key)
          return [
            <GridActionsCellItem icon={<WaterDamage></WaterDamage>} onClick={() => window.navigator.clipboard.writeText(rows[params.id as number].leaked)} label="Create New Message" />,
          ];
        else
          return []
      }
    }
  ];

  return <DataGrid rows={rows} columns={columns} components={{ Toolbar: CustomToolbar(props.url) }} />;

}
function CustomToolbar(url: string) {
  return () => (
    <GridToolbarContainer>
      <h4>Chains</h4>
      <MakeGenesis url={url}></MakeGenesis>
      <MakeGenesisImported url={url}></MakeGenesisImported>
    </GridToolbarContainer>
  );
}
