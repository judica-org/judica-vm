import { ContentCopy } from '@mui/icons-material';
import { Button, FormControl, IconButton, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { FormEvent, useEffect, useState } from "react";
import { tauri_host } from '../tauri_host';

export interface KeySelectorProps {
  signing_key: string | null,
  available_keys: string[]
};
export function KeySelector({ signing_key, available_keys }: KeySelectorProps) {
  const [selected_key, set_selected_key] = useState<string>(signing_key??"");

  const handle_submit = (ev: FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    console.log(["selected-key"], selected_key);
    // redundant but more clear to check both
    if (selected_key || selected_key !== "") tauri_host.set_signing_key(selected_key);
    else tauri_host.set_signing_key(null);
  };

  // reset selected key
  // reset selected key
  // if (selected_key && new_keys.indexOf(selected_key) == -1) {
  //   tauri_host.set_signing_key(null);
  //   set_selected_key("");
  // }
  let key_options = available_keys.map((key) => {
    return <MenuItem value={key} selected={key === selected_key} key={key}>{key}</MenuItem>;
  })

  return <div>
    <h6>Select Player</h6>
    <FormControl >
      <Select label="Public Key" onChange={(ev) => set_selected_key(ev.target.value as string)} value={selected_key} renderValue={(v) => `${v.substring(0, 16)}...`}>
        <MenuItem value={""} selected={selected_key === ""}>No Key</MenuItem>
        {key_options}
      </Select>
      <Button variant="contained" type="submit" onClick={handle_submit}>Select This Key</Button>
      {signing_key && <IconButton onClick={() => window.navigator.clipboard.writeText(signing_key)}><ContentCopy></ContentCopy></IconButton>}
    </FormControl>
  </div>
}