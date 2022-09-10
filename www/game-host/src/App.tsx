import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';

function App() {
  return (
    <div className="App">
      <div>

        <ListGames></ListGames>
        <NewGame></NewGame>
      </div>
      <div>
        <ListPeers></ListPeers>
        <AddPeerToNode></AddPeerToNode>
      </div>
      <div>
        <AddChainToGroup></AddChainToGroup>

      </div>
    </div>
  );
}
type CreatedNewChain = {
  genesis_hash: string,
  group_name: string,
};
function NewGame() {
  async function handle_click() {
    let res = await fetch("http://127.0.0.1:11409/attestation_chain/new",
      { method: "POST" });

    let js = await res.json() as CreatedNewChain;
    console.log(js);

  }
  return <button onClick={handle_click}>New Game</button>
}

function AddPeerToNode() {
  async function handle_click(ev: FormEvent) {
    ev.preventDefault();
    let t = ev.target as typeof ev.target & {
      service_url: { value: string },
      port: { valueAsNumber: number }
    };
    let obj = {
      service_url: t.service_url,
      port: t.port,
    };
    let res = await fetch("http://127.0.0.1:11409/peer",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(obj)
      });

    let js = await res.json();
    console.log(js);

  }
  return <form onSubmit={handle_click}>
    <label>Service URL</label>
    <input name="service_url" type="text" />
    <label>Port</label>
    <input name="genesis_hash" type="number" />
    <button type="submit">Add Peer to Node</button>
  </form>
}
function AddChainToGroup() {
  async function handle_click(ev: FormEvent) {
    ev.preventDefault();
    let t = ev.target as typeof ev.target & {
      group: { value: string },
      genesis_hash: { value: string }
    };
    let obj = {
      group: t.group,
      genesis_hash: t.genesis_hash,

    };
    let res = await fetch("http://127.0.0.1:11409/attestation_chain/commit_group/add_member",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(obj)
      });

    let js = await res.json();
    console.log(js);

  }
  return <form onSubmit={handle_click}>
    <label>Group</label>
    <input name="group" type="text" />
    <label>Genesis Hash</label>
    <input name="genesis_hash" type="text" />
    <button type="submit">Add Chain To Group</button>
  </form>
}

function ListGames() {
  const [list_of_games, set_list_of_games] = React.useState([]);
  React.useEffect(() => {
    let cancel = false;
    const updater = async () => {
      if (cancel) return;
      let res = await fetch("http://127.0.0.1:11409/attestation_chain");
      let js = await res.json();
      console.log(js);
      set_list_of_games(js);
      setTimeout(() => updater(), 1000);
    }
    updater();
    return () => { cancel = true };
  }, []);
  const games = list_of_games.map((m) => <li> {m}</li>);
  return <div>
    <h2>Game Room IDs</h2>
    <ul>
      {games}
    </ul>
  </div>
}

function ListPeers() {
  const [list_of_peers, set_list_of_peers] = React.useState<Array<{ service_url: string, port: number }>>([]);
  React.useEffect(() => {
    let cancel = false;
    const updater = async () => {
      if (cancel) return;
      let res = await fetch("http://127.0.0.1:11409/peer");
      let js = await res.json();
      console.log(js);
      set_list_of_peers(js);
      setTimeout(() => updater(), 1000);
    }
    updater();
    return () => { cancel = true };
  }, []);
  const games = list_of_peers.map((m) => <li> {`${m.service_url}:${m.port}`}</li>);
  return <div>
    <h2>Game Room IDs</h2>
    <ul>
      {games}
    </ul>
  </div>
}


export default App;
