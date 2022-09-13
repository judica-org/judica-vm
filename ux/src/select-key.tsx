import { appWindow } from '@tauri-apps/api/window';
import { FormEvent, useEffect, useState } from "react";
import { tauri_host } from './tauri_host';

export const KeySelector = () => {
  const [selected_key, set_selected_key] = useState<string | null>(null);
  const [signing_key, set_signing_key] = useState<string | null>(null);
  const [available_keys, set_available_keys] = useState<string[]>([]);

  useEffect(() => {
    const unlisten = appWindow.listen("user-keys", (ev) => {
      console.log(ev.payload);
      const new_keys = ev.payload as typeof available_keys;
      // reset selected key
      if (selected_key && new_keys.indexOf(selected_key) == -1) {
        tauri_host.set_signing_key(null);
        set_selected_key(null);
      }
      set_available_keys(new_keys);
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });

  useEffect(() => {
    const unlisten = appWindow.listen("signing-key", (ev) => {
      console.log(["signing_key"], ev);
      set_signing_key(ev.payload as string)
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });

  const handle_submit = (ev: FormEvent<HTMLFormElement>): void => {
    ev.preventDefault();
    console.log(["selected_key"], selected_key);
    // redundant but more clear to check both
    if (selected_key || selected_key !== "") tauri_host.set_signing_key(selected_key);
    else tauri_host.set_signing_key(null);
  };

  let key_options = available_keys.map((key) => {
    return <option value={key} selected={key === selected_key}>{key}</option>;
  })

  return <div>
    <h4>Signing With: {signing_key}</h4>
    <form onSubmit={handle_submit}>
      <label>Pub Key</label>
      <select onChange={(ev) => set_selected_key(ev.target.value)}>
        <option value={""} selected={selected_key == null}>No Key</option>
        {key_options}
      </select>
      <button type="submit">Select This Key</button>
    </form>
  </div>
}