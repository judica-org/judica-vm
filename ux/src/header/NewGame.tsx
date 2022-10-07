import { Button, FormControl, Slider, TextField, ToggleButton } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export interface NewGameProps  {
  join_code: string|null,
  join_password: string|null
};
export function NewGame(props: NewGameProps) {
  const [nick, set_nick] = React.useState<null | string>(null);

  const [join_or_new, set_join_or_new] = React.useState(false);
  return <div>
    <h6> New Player</h6>
    <FormControl >
      <ToggleButton value={join_or_new} onChange={(a)=> {set_join_or_new(!join_or_new)}}></ToggleButton>
      <TextField label='Nickname' onChange={(ev) => set_nick(ev.target.value)}></TextField>
      <Button variant="contained" type="submit" onClick={(ev) => { ev.preventDefault(); nick && tauri_host.make_new_chain(nick) }}>New Game</Button>
    </FormControl>
  </div>;
}
