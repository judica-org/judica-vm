// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Button, FormGroup, FormLabel, MenuItem, Select } from '@mui/material';
import React from 'react';
import { tauri_host } from '../tauri_host';
import { GameSetup } from '../Types/Gameboard';


export interface SwitchToGameProps {
  available_sequencers: [string, GameSetup][];
  which_game_loaded: null | string;
};

export function SwitchToGame({ available_sequencers, which_game_loaded }: SwitchToGameProps) {
  const [which_game, set_which_game] = React.useState<string | 0>(which_game_loaded ?? 0);

  let options = available_sequencers.map(([pkey, name]) => {
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
        <MenuItem value={0} selected={which_game === 0}>None</MenuItem>
        {options}
      </Select>
      <Button type="submit" variant="contained"
        onClick={handle_submit}
      >Switch Game</Button>
    </FormGroup>
  </div>;
}
