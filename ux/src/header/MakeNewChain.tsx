import { Button, FormControl, TextField } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export function MakeNewChain() {
  const [nick, set_nick] = React.useState<null | string>(null);


  return <div>
    <h6> Create New Chain</h6>
    <FormControl >
      <TextField label='Nickname' onChange={(ev) => set_nick(ev.target.value)}></TextField>
      <Button variant="contained" type="submit" onClick={(ev) => { ev.preventDefault(); nick && tauri_host.make_new_chain(nick) }}>Create New Chain</Button>
    </FormControl>
  </div>;
}
