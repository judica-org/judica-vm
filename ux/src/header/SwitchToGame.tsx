import { ContentCopy } from '@mui/icons-material';
import { Button, FormControl, FormGroup, FormLabel, IconButton, InputLabel, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export type GameSetup = {}

export interface SwitchToGameProps {
  available_sequencers: [string, GameSetup][];
  which_game_loaded: null | string;
};

export function SwitchToGame({ available_sequencers, which_game_loaded }: SwitchToGameProps) {
  const [which_game, set_which_game] = React.useState<string>(which_game_loaded ?? "");

  let options = available_sequencers.map(([pkey, name]) => {
    return <MenuItem value={pkey} key={pkey}>
      {pkey}
    </MenuItem>;
  });
  const handle_submit = (ev: React.FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    which_game && tauri_host.switch_to_game(which_game);
  };
  return <div>
    <FormLabel>Existing Game</FormLabel>
    <FormGroup>
      <Select label="Game Key" onChange={(ev) => set_which_game(ev.target.value as string)} value={which_game} renderValue={(v) => `${v.substring(0, 16)}...`}>
        <MenuItem value={""} selected={which_game == ""}>No Key</MenuItem>
        {options}
      </Select>
      <Button type="submit" variant="contained"
        onClick={handle_submit}
      >Switch Game</Button>
      {which_game_loaded && <IconButton onClick={() => window.navigator.clipboard.writeText(which_game_loaded)}><ContentCopy></ContentCopy></IconButton>}
    </FormGroup>
  </div>;
}
