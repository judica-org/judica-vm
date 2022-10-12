// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Report } from '@mui/icons-material';
import { Button, Table, TableBody, TableCell, TableHead, TableRow, Typography } from '@mui/material';
import { Envelope } from './App';
import "./ChainCommitGroups.css";

type ChainCommitGroups = {
  genesis: string,
  members: Array<Envelope>,
  all_msgs: Record<string, Envelope>
};
export function ChainCommitGroups(props: { url: string; genesis: string }) {
  const [data, set_data] = React.useState<ChainCommitGroups>({ genesis: props.genesis, members: [], all_msgs: {} });
  const [text, set_text] = React.useState("");
  const handle = async () => {
    const target = `${props.url}/chain_commit_groups`;
    console.log("Fetching...", target);
    try {
      const resp = await fetch(target,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(props.genesis)
        });
      const js = await resp.json();
      set_data(js);
    }
    catch (e) {
      console.warn(e);
    }
  };
  function CustomToolbar() {
    return (
      <GridToolbarContainer>
        <h4>All Messages in DB</h4>
        <Button onClick={handle}> Refresh </Button>
      </GridToolbarContainer>
    );
  }

  const rows: GridRowsProp = Object.entries(data.all_msgs).map(([msg_hash, envelope]: [string, Envelope], id) => {
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

  return <div className="CCGGrid">

    <Typography variant='h4' sx={{ gridArea: "header" }}>Selected {props.genesis}</Typography>
    <Table sx={{gridArea:"members"}}>
      <TableHead>
        <TableRow>
          <TableCell>Genesis</TableCell>
          <TableCell>Key</TableCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {data.members.map((member) => {
          return <TableRow>
            <TableCell>{member.header.ancestors?.genesis ?? "TODO: Pass Message Hash"}</TableCell>
            <TableCell>{member.header.key}</TableCell>
          </TableRow>
        })}
      </TableBody>
    </Table>
    <DataGrid sx={{ gridArea: "all_msgs" }} rows={rows} columns={columns} components={{ Toolbar: CustomToolbar }} />

  </div>;
}
