import { ContentCopy } from '@mui/icons-material';
import { Button, FormControl, IconButton, MenuItem, Select } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { FormEvent, useEffect, useState } from "react";
import { tauri_host } from '../tauri_host';

export const KeySelector = () => {
  const [selected_key, set_selected_key] = useState<string>("");
  const [signing_key, set_signing_key] = useState<string | null>(null);
  const [available_keys, set_available_keys] = useState<string[]>([]);

  useEffect(() => {
    const unlisten = appWindow.listen("user-keys", (ev) => {
      console.log(["available keys"], ev.payload);
      const new_keys = ev.payload as typeof available_keys;
      // reset selected key
      if (selected_key && new_keys.indexOf(selected_key) == -1) {
        tauri_host.set_signing_key(null);
        set_selected_key("");
      }
      set_available_keys(new_keys);
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  }, []);

  useEffect(() => {
    const unlisten = appWindow.listen("signing-key", (ev) => {
      console.log(["signing-key"], ev.payload);
      set_signing_key(ev.payload as string)
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  }, []);

  const handle_submit = (ev: FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    console.log(["selected-key"], selected_key);
    // redundant but more clear to check both
    if (selected_key || selected_key !== "") tauri_host.set_signing_key(selected_key);
    else tauri_host.set_signing_key(null);
  };

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
      {selected_key && <IconButton onClick={() => window.navigator.clipboard.writeText(selected_key)}><ContentCopy></ContentCopy></IconButton>}
    </FormControl>
  </div>
}