import { ContentCopy } from '@mui/icons-material';
import { Button, FormControl, FormGroup, FormLabel, IconButton, InputLabel, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export type GameSetup = {
  players: Array<string>,
  start_amount: number,
  finish_time: number,
}

export interface SwitchToGameProps {
  available_sequencers: [string, GameSetup][];
  which_game_loaded: null | string;
};

export function SwitchToGame({ available_sequencers, which_game_loaded }: SwitchToGameProps) {
  const [which_game, set_which_game] = React.useState<string | 0>(which_game_loaded ?? 0);

  let options = available_sequencers.map(([pkey, name]) => {
    console.log("OPT", pkey, which_game_loaded);
    return <MenuItem value={pkey} key={pkey}>
      {pkey}
    </MenuItem>;
  });
  const handle_submit = async (ev: React.FormEvent<HTMLButtonElement>): Promise<void> => {
    ev.preventDefault();
    if (which_game !== 0) {
      console.log("SWITCHING TO", which_game);
      await tauri_host.switch_to_game(which_game);
      console.log("DONE SWITCHING");
    }
  };
  return <div >
    <FormLabel>Existing Game</FormLabel>
    <FormGroup>
      <Select label="Game Key"
        onChange={(ev) => set_which_game(ev.target.value as string)}
        value={which_game}
        renderValue={(v) => `${(v && v || null)?.substring(0, 16) ?? "None"}...`}
      >
        <MenuItem value={0} selected={which_game === 0}></MenuItem>
        {options}
      </Select>
      <Button type="submit" variant="contained"
        onClick={handle_submit}
      >Switch Game</Button>
      {which_game_loaded && <IconButton onClick={() => window.navigator.clipboard.writeText(which_game_loaded)}><ContentCopy></ContentCopy></IconButton>}
    </FormGroup>
  </div>;
}
