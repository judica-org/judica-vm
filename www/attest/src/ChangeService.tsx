// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import React, { FormEvent } from 'react';
import { alpha, Button, FormControl, FormGroup, TextField, useTheme } from '@mui/material';

export function ChangeService({ set_url }: { set_url: (arg0: string) => void; }) {
  const theme = useTheme();
  const [service, set_service] = React.useState<string>("");
  const handle_click = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {
    ev.preventDefault();
    console.log(service);
    const url = new URL(global.location.toString());
    url.searchParams.set("service_url", service);
    console.log(url.toString());
    global.location.href = url.toString();
    set_url(service);
  };
  return <FormControl size="small">
    <FormGroup row={true}>
      <TextField variant="filled" color="success" size="small" onChange={(ev) => set_service(ev.target.value)} name="service" type="text"
        style={{
          backgroundColor: alpha(theme.palette.common.white, 0.65),
        }}
      ></TextField>
      <Button variant="contained" color="success" type="submit" onClick={handle_click}>Set Server</Button>
    </FormGroup>
  </FormControl>;
}
