import React from 'react';
import { tauri_host } from './tauri_host';

export function SwitchToGame() {
  const [which_game, set_which_game] = React.useState<string | null>(null);

  const handle_submit = (ev: React.FormEvent<HTMLFormElement>): void => {
    ev.preventDefault();
    which_game && tauri_host.switch_to_game(which_game);
  };
  return <div>
    <form onChange={handle_submit}>
      <label>Game Key</label>
      <input type="text" onChange={(ev) => set_which_game(ev.target.value)}></input>
      <button type="submit">Switch</button>
    </form>
  </div>;
}
