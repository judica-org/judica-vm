import { Button, FormControl, FormGroup, InputLabel, TextField } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export function SwitchToDB(props: { db_name_loaded: [string, string | null] | null, set_db_name_loaded: ((arg: [string, string | null] | null) => void) }) {
  const [db_prefix, set_db_prefix] = React.useState<string | null>(null);
  const [db_appname, set_db_appname] = React.useState<string | null>(null);
  React.useEffect(() => {
    const unlisten = appWindow.listen("db-connection", (ev) => {
      console.log(ev);
      props.set_db_name_loaded(ev.payload as ([string, string | null] | null));
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });


  const handle_submit = (ev: React.FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    // prefix allowed to be null
    db_appname && tauri_host.switch_to_db(db_appname, db_prefix);
  };
  return <div>
    <h6>Loaded DB:{props.db_name_loaded && `${props.db_name_loaded[0]} ${props.db_name_loaded[1]}`}</h6>
    <FormControl>
      <TextField label="FS Prefix" required={false} onChange={(ev) => set_db_prefix(ev.target.value)}></TextField>
      <TextField label="Name" required={true} onChange={(ev) => set_db_appname(ev.target.value)}></TextField>
      <Button variant="contained" type="submit"
        onClick={handle_submit}
      >Switch DB</Button>
    </FormControl>
  </div>;
}
