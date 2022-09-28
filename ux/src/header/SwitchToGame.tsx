import { Button, FormControl, InputLabel, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export function SwitchToGame() {
  const [which_game, set_which_game] = React.useState<string>("");

  const [which_game_loaded, set_which_game_loaded] = React.useState<string | null>(null);
  const [available_sequencers, set_available_sequencers] = React.useState<Array<[string, string]>>([]);
  React.useEffect(() => {
    const unlisten = appWindow.listen("available-sequencers", (ev) => {
      console.log(ev.payload);
      set_available_sequencers(ev.payload as typeof available_sequencers);
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });
  React.useEffect(() => {
    const unlisten = appWindow.listen("host-key", (ev) => {
      console.log(ev.payload);
      set_which_game_loaded(ev.payload as string);
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });
  let options = available_sequencers.map(([pkey, name]) => {
    return <MenuItem value={pkey} key={pkey}>
      {name}
    </MenuItem>;
  });
  const handle_submit = (ev: React.FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    which_game && tauri_host.switch_to_game(which_game);
  };
  return <div>
    <h6>Sequencer: {which_game_loaded}</h6>
    <FormControl fullWidth>
      <InputLabel>Game Key</InputLabel>
      <Select onChange={(ev) => set_which_game(ev.target.value as string)} value={which_game}>
        <MenuItem value={""} selected={which_game == ""}>No Key</MenuItem>
        {options}
      </Select>
      <Button type="submit" variant="contained"
        onClick={handle_submit}
      >Switch Game</Button>
    </FormControl>
  </div>;
}
