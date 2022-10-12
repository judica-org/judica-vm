// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Button, FormControl,  FormLabel, TextField } from '@mui/material';
import React from 'react';
import { tauri_host } from '../tauri_host';

export function SwitchToDB(props: { db_name_loaded: [string, string | null] | null }) {
  const [db_prefix, set_db_prefix] = React.useState<string | null>(null);
  const [db_appname, set_db_appname] = React.useState<string | null>(null);


  const handle_submit = (ev: React.FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    // prefix allowed to be null
    db_appname && tauri_host.switch_to_db(db_appname, db_prefix);
  };
  return <FormControl>
    <FormLabel>Loaded DB:{props.db_name_loaded && ` ${props.db_name_loaded[1] ?? ""} ${props.db_name_loaded[0]}`}</FormLabel>
    <TextField label="FS Prefix" required={false} onChange={(ev) => set_db_prefix(ev.target.value)}></TextField>
    <TextField label="Name" required={true} onChange={(ev) => set_db_appname(ev.target.value)}></TextField>
    <Button variant="contained" type="submit"
      onClick={handle_submit}
    >Switch DB</Button>
  </FormControl>;
}
