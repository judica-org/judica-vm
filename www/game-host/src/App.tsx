import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';

type AddChainToGroupArgs = {
  group: string,
  genesis_hash: string

};
class Client {
  base_url: string;
  constructor() {
    const start = new URL(global.location.toString());
    this.base_url =
      start.searchParams.get("service_url") ?? "";
    if (this.base_url === "")
      throw "Service URL Required";
  }
  async create_new_chain(): Promise<CreatedNewChain> {

    let res = await fetch(`${this.base_url}/attestation_chain/new`,
      {
        method: "POST"
      });

    let js = await res.json() as CreatedNewChain;
    return js;
  }
  async add_chain_to_group(obj: AddChainToGroupArgs): Promise<void> {

    let res = await fetch(`${this.base_url}/attestation_chain/commit_group/add_member`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(obj)
      });

    let js = await res.json();
    console.log(js);
  }
  async add_peer(obj: { service_url: string, port: number }): Promise<void> {
    let res = await fetch(`${this.base_url}/peer`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(obj)
      });

    let js = await res.json();
    console.log(js);
    return;
  }
  async list_games(): Promise<Array<string>> {
    let res = await fetch(`${this.base_url}/attestation_chain`);
    let js = await res.json();
    console.log(js);
    return js;
  }
  async list_peers(): Promise<Array<Peer>> {
    let res = await fetch(`${this.base_url}/peer`);
    let js = await res.json();
    return js;
  }
}
const client = new Client();
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
    const js = await client.create_new_chain();
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
      service_url: t.service_url.value,
      port: t.port.valueAsNumber,
    };
    const js = client.add_peer(obj);
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
      group: t.group.value,
      genesis_hash: t.genesis_hash.value,

    };
    client.add_chain_to_group(obj);

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
  const [list_of_games, set_list_of_games] = React.useState<Array<string>>([]);
  React.useEffect(() => {
    let cancel = false;
    const updater = async () => {
      if (cancel) return;
      let js = await client.list_games();
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

type Peer = { service_url: string, port: number };
function ListPeers() {
  const [list_of_peers, set_list_of_peers] = React.useState<Array<Peer>>([]);
  React.useEffect(() => {
    let cancel = false;
    const updater = async () => {
      if (cancel) return;
      const js = await client.list_peers();
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
