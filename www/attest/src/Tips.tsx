// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Diversity3, ReadMore } from '@mui/icons-material';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp } from '@mui/x-data-grid';
import React from 'react';
import { Envelope } from './App';

type TipProps = {
  tips: Array<{
    envelope: Envelope;
    hash: string;
  }>;
  set_genesis: (arg: null | string) => void;
};

export function Tips(props: TipProps) {
  const [view_flash, flash] = React.useState<null | string>(null);
  React.useEffect(() => {
    const t = setTimeout(() => view_flash && flash(null), 1000);
  }, [view_flash]);
  const copy_on_click = (s: string) => {
    return (ev: React.MouseEvent<HTMLTableDataCellElement, MouseEvent>) => {
      flash(`Copied ${s}`);
      navigator.clipboard.writeText(s);
    };
  };

  const rows: GridRowsProp = props.tips.map((tip, id) => {
    const genesis = tip.envelope.header.ancestors?.genesis ?? tip.hash;
    const msg = JSON.stringify(tip.envelope.msg);
    const signing_key = tip.envelope.header.key;
    return { id, genesis, msg, signing_key, hash: tip.hash, height: tip.envelope.header.height };
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
        <GridActionsCellItem icon={<ReadMore></ReadMore>} onClick={() => console.log(["message"], JSON.parse(rows[params.id as number].msg))} label="Log Message" />,
        <GridActionsCellItem icon={<Diversity3></Diversity3>} onClick={() => props.set_genesis(rows[params.id as number].genesis)} label="Set Genesis" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} />;
}
