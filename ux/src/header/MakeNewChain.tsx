import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import { tauri_host } from '../tauri_host';

export function MakeNewChain() {
  const [nick, set_nick] = React.useState<null | string>(null);


  return <div>
    <h6> Create New Chain</h6>
    <input type="text" placeholder='Nickname' onChange={(ev) => set_nick(ev.target.value)}></input>
    <button onClick={() => { nick && tauri_host.make_new_chain(nick) }}>Create New Chain</button>
  </div>;
}
