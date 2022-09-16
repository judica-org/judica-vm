import React from 'react';
import { DataGrid, GridActionsCellItem, GridRowParams, GridRowsProp } from '@mui/x-data-grid';
import { Report } from '@mui/icons-material';
import { Envelope } from './App';

export function Tips(props: { tips: Array<{ envelope: Envelope; hash: string; }>; }) {
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
        <GridActionsCellItem icon={<Report></Report>} onClick={() => console.log(["message"], JSON.parse(rows[params.id as number].msg))} label="Log Message" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} />;
}
