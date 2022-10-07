import { Button, FormControl, FormControlLabel, Slider, Switch, TextField, ToggleButton, Typography } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export interface NewGameProps {
  join_code: string | null,
  join_password: string | null
};
export function NewGame({ join_code, join_password }: NewGameProps) {
  const [nick, set_nick] = React.useState<null | string>(null);

  const [join_or_new, set_join_or_new] = React.useState(false);
  const action = join_or_new ? "Join" : "New";
  const handle_click = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {
    ev.preventDefault();
    if (join_or_new) {
      nick && tauri_host.make_new_game(nick);
    } else {
      nick && tauri_host.make_new_chain(nick);
    }
  };
  return <div>
    <Typography variant='h6'>New Game</Typography>
    <FormControl >
      <FormControlLabel control={
        <Switch value={join_or_new} onClick={(a) => { set_join_or_new(!join_or_new) }}></Switch>
      } label={`${action} Game`} />

      <TextField label='Chain Nickname' onChange={(ev) => set_nick(ev.target.value)}></TextField>
      {
        join_or_new && <TextField label='Join Code' onChange={(ev) => set_nick(ev.target.value)}></TextField>
      }
      <Button variant="contained" type="submit" onClick={handle_click}>
        {action} Game
      </Button>
      {
        join_code && <Typography>{join_code}</Typography>
      }
      {
        join_password && <Typography>{join_password}</Typography>
      }
    </FormControl>
  </div>;
}
