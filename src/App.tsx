import { invoke } from '@tauri-apps/api';
import { appWindow } from '@tauri-apps/api/window';
import React from 'react';
import './App.css';
import logo from './logo.svg';
function App() {
  const [game_board, set_game_board] = React.useState<unknown>({});
  React.useEffect(() => {
    const unlisten = appWindow.listen("game-board", (ev) => {
      set_game_board(ev.payload)
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
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          {JSON.stringify(game_board)}
        </p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
}

export default App;
