import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

let lastId = 0;
function unique_id(prefix = 'switch-to-db-id') {
  lastId++;
  return `${prefix}${lastId}`;
}
export function SwitchToDB() {
  const [db_prefix, set_db_prefix] = React.useState<string | null>(null);
  const [db_appname, set_db_appname] = React.useState<string | null>(null);
  const id_prefix = React.useRef(unique_id());
  const id_appname = React.useRef(unique_id());
  const [db_name_loaded, set_db_name_loaded] = React.useState<[string, string | null]>(["", null]);
  React.useEffect(() => {
    const unlisten = appWindow.listen("db-connection", (ev) => {
      console.log(ev);
      set_db_name_loaded(ev.payload as [string, string | null]);
    })
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });


  const handle_submit = (ev: React.FormEvent<HTMLFormElement>): void => {
    ev.preventDefault();
    // prefix allowed to be null
    db_appname && tauri_host.switch_to_db(db_appname, db_prefix);
  };
  return <div>
    <h4>Connected To:{db_name_loaded[0]} {db_name_loaded[1]}</h4>
    <form onSubmit={handle_submit}>
      <label htmlFor={id_prefix.current}>DB Prefix</label>
      <input id={id_prefix.current} type="text" required={false} onChange={(ev) => set_db_prefix(ev.target.value)}></input>

      <label htmlFor={id_appname.current}>App Name</label>
      <input id={id_appname.current} type="text" required={true} onChange={(ev) => set_db_appname(ev.target.value)}></input>
      <button type="submit">Switch DB</button>
    </form>
  </div>;
}
