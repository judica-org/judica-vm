import { invoke } from '@tauri-apps/api';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import './App.css';
import logo from './logo.svg';
import Form from "@rjsf/core";


function MoveForm() {
  const [schema, set_schema] = React.useState<null | any>(null);
  React.useEffect(() => {
    (async () => {
      set_schema(await invoke("get_move_schema"));
    })()
  });
  return schema && <div className='MoveForm'><Form schema={schema}></Form></div>;
}

type game_board = {
  erc20s: any,
  swap: any,
  turn_count: number,
  alloc: any,
  users: Record<string, string>,
  nfts: any,
  nft_sales: any,
  player_move_sequences: Record<string, number>,
  init: boolean,
  new_users_allowed: boolean,
  bitcoin_token_id: null | string,
  dollar_token_id: null | string,
  root_user: null | string,
};
function GameBoard(props: { g: game_board }) {
  return <ul>
    <li>
      Init: {JSON.stringify(props.g.init)}
    </li>
    <li>
      New Users: {JSON.stringify(props.g.new_users_allowed)}
    </li>
    <li>
      User List: {JSON.stringify(props.g.users)}
    </li>
    <li>
      Root User: {JSON.stringify(props.g.root_user)}
    </li>
    <li>
      Bitcoin Token ID: {JSON.stringify(props.g.bitcoin_token_id)}
    </li>
    <li>
      Dollar Token ID: {JSON.stringify(props.g.dollar_token_id)}
    </li>
    <li>
      ERC20s: {JSON.stringify(props.g.erc20s)}
    </li>
    <li>
      Exchanges: {JSON.stringify(props.g.swap)}
    </li>
    <li>
      NFTs: {JSON.stringify(props.g.nfts)}
    </li>
    <li>
      NFT Sales: {JSON.stringify(props.g.nft_sales)}
    </li>


  </ul>;
}
function App() {
  const [game_board, set_game_board] = React.useState<game_board | null>(null);
  React.useEffect(() => {
    const unlisten = appWindow.listen("game-board", (ev) => {
      console.log(ev);
      set_game_board(JSON.parse(ev.payload as string) as game_board)
    });
    invoke("game_synchronizer")
    return () => {
      (async () => {
        (await unlisten)()
      })();
    }
  });

  return (
    <div className="App">
      {game_board && <GameBoard g={game_board}></GameBoard>}
      <MoveForm></MoveForm>
    </div>
  );
}

export default App;
