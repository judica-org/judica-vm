import React from 'react';
import logo from './logo.svg';
import './App.css';

function App() {
  return (
    <div className="App">
      <ListGames></ListGames>
      <NewGame></NewGame>
    </div>
  );
}
type CreatedNewChain = {
  genesis_hash: string,
  group_name: string,
};
function NewGame() {
  async function handle_click() {
    let res = await fetch("http://127.0.0.1:13329/attestation_chain/new",
      { method: "POST" });

    let js = await res.json() as CreatedNewChain;
    console.log(js);

  }
  return <button onClick={handle_click}>New Game</button>
}

function ListGames() {
  const [list_of_games, set_list_of_games] = React.useState([]);
  React.useEffect(() => {
    let cancel = false;
    const updater = async () => {
      if (cancel) return;
      let res = await fetch("http://127.0.0.1:13329/attestation_chain");
      let js = await res.json();
      console.log(js);
      set_list_of_games(js);
      setTimeout(() => updater(), 5000);
    }
    updater();
    return () => { cancel = true };
  }, []);
  const games = list_of_games.map((m) => <li> {m}</li>);
  return <div>
    <ul>
      {games}
    </ul>
  </div>
}


export default App;
