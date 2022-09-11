import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from './tauri_host';

export function SwitchToGame() {
  const [which_game, set_which_game] = React.useState<string | null>(null);

  const [which_game_loaded, set_which_game_loaded] = React.useState<string | null>(null);
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
  const handle_submit = (ev: React.FormEvent<HTMLFormElement>): void => {
    ev.preventDefault();
    which_game && tauri_host.switch_to_game(which_game);
  };
  return <div>

    <h4>Connected To: {which_game_loaded}</h4>
    <form onSubmit={handle_submit}>
      <label>Game Key</label>
      <input type="text" onChange={(ev) => set_which_game(ev.target.value)}></input>
      <button type="submit">Switch</button>
    </form>
  </div>;
}
