import { ContentCopy } from '@mui/icons-material';
import { Button, FormControl, FormLabel, IconButton, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { FormEvent, useEffect, useState } from "react";
import { tauri_host } from '../tauri_host';
import { GameSetup } from '../Types/Gameboard';

export interface KeySelectorProps {
  signing_key: string | null,
  available_keys: string[],
  available_sequencers: [string, GameSetup][],
  which_game_loaded: string|null;
};

export interface KeySelectorDirectProps {
  disabled: boolean
}
export function KeySelector({ which_game_loaded, available_sequencers, signing_key, available_keys, disabled }: KeySelectorProps & KeySelectorDirectProps) {

  const setup = available_sequencers.find((([key, _setup]) => key === which_game_loaded));
  const usable_keys = (setup && available_keys.filter((k) => setup[1].players.includes(k))) ?? [];

  console.log("USABLE KEYS", usable_keys);
  const [selected_key, set_selected_key] = useState<string|0>(signing_key ?? 0);

  const handle_submit = (ev: FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    console.log(["selected-key"], selected_key);
    // redundant but more clear to check both
    if (selected_key !== 0) tauri_host.set_signing_key(selected_key);
    else tauri_host.set_signing_key(null);
  };

  // reset selected key
  // reset selected key
  // if (selected_key && new_keys.indexOf(selected_key) == -1) {
  //   tauri_host.set_signing_key(null);
  //   set_selected_key("");
  // }
  let key_options = usable_keys.map((key) => {
    return <MenuItem value={key} selected={key === selected_key} key={key}>{key}</MenuItem>;
  })

  if (setup)
    return <div>
      <FormControl disabled={disabled}>
        <FormLabel>Select Player</FormLabel>
        <Select label="Public Key" 
        onChange={(ev) => set_selected_key(ev.target.value as string)} 
        value={selected_key} 
        renderValue={(v) => `${(v !== 0? v : null)?.substring(0, 16)??"None"}...`}>
          <MenuItem value={0} selected={selected_key === 0} ></MenuItem>
          {key_options}
        </Select>
        <Button variant="contained" type="submit" onClick={handle_submit}>Select This Key</Button>
        {signing_key && <IconButton onClick={() => window.navigator.clipboard.writeText(signing_key)}><ContentCopy></ContentCopy></IconButton>}
      </FormControl>
    </div>
  else
    return <div></div>
}