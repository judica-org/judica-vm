import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';


function App() {
  const start = new URL(global.location.toString());
  const init = start.searchParams.get("service_url");
  const service = React.useRef<HTMLInputElement | null>(null);
  const [url, set_url] = React.useState<null | string>(init);
  const [status, set_status] = React.useState<null | any>(null);
  React.useEffect(
    () => {
      let cancel = false;
      async function fetcher() {
        if (cancel) return;
        if (!url) return;
        const target = `${url}/status`;
        console.log("Fetching...", target);
        try {
          const resp = await fetch(target);
          const js = await resp.json();
          console.log(js);
          set_status(js);
          setTimeout(fetcher, 5000)
        } catch {
          set_url(null);
        }
      }
      fetcher();
      return () => {
        cancel = true;
      }
    }
    , [url])
  const handle_click = (ev: FormEvent) => {
    ev.preventDefault();
    if (service.current !== null) {
      const url = new URL(global.location.toString());
      url.searchParams.set("service_url", service.current.value)
      global.location.href = url.toString();
      set_url(service.current.value)
    }
  };
  return (
    <div className="App" >
      Connected to: {url}
      <form onSubmit={handle_click}>
        <input ref={service} name="service" type="text" ></input>
        <button type="submit">Set Server</button>
      </form>
      <hr></hr>
      {url && <AddPeer root={url}></AddPeer>}
      <hr></hr>
      {url && <MakeGenesis url={url}></MakeGenesis>}
      <hr></hr>
      {status && status.hidden_service_url && <div>
        <h1>Tor Enabled:</h1>
        <h2>{status.hidden_service_url[0]}:{status.hidden_service_url[1]} </h2>
      </div>}
      {status && <Peers peers={status.peers}></Peers>}
      {status && <TaskSet tasks={status.peer_connections}></TaskSet>}
      {status && <Tips tips={status.tips}></Tips>}
      {status && url && <Users users={status.all_users} url={url}></Users>}
      {url && <ExpensiveMsgDB url={url}></ExpensiveMsgDB>}

    </div>
  );
}

function MakeGenesis(props: { url: String }) {
  const handle = async () => {
    const new_genesis = window.prompt("New Genesis Named?");
    if (new_genesis) {

      const ret = fetch(`${props.url}/make_genesis`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(new_genesis)
      })
      console.log(await (await ret).json());
    }
  };
  return <button onClick={() => handle()}>New Genesis</button>;
}

function NewMsg(props: { url: String, pk: String }) {
  const handle = async () => {
    const message = window.prompt("DANGER: Invalid message May Corrupt Your Chain.\n\nWhat message should we send?");
    if (message) {
      let js = JSON.parse(message);
      const c = window.confirm(`Are you sure? Pushing: \n ${JSON.stringify(message)}`);
      if (!c) return;
      const ret = fetch(`${props.url}/push_message_dangerous`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ msg: js, key: props.pk })
      })
      console.log(await (await ret).json());
    }
  };
  return <button onClick={() => handle()}>Create Message</button>;
}

function Users(props: { users: Array<[String, String, boolean]>, url: String }) {

  const rows = props.users.map(([pubkey, nickname, has_priv]) => <tr>
    <td>{pubkey}</td>
    <td>{nickname}</td>
    <td>{has_priv ? "Yes" : "No"}</td>
    <td>{has_priv && <NewMsg url={props.url} pk={pubkey}></NewMsg>}</td>
  </tr>);
  return <table>
    <thead>
      <tr>
        <th>Key</th>
        <th>Nickname</th>
        <th>Private Key Known?</th>
        <th>Push Message</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
}
function Tips(props: { tips: Array<{ envelope: { header: { ancestors?: { genesis: string }, height: string }, msg: any }, hash: string }> }) {

  const rows = props.tips.map((x) => <tr key={x.hash}>
    <td>{x.envelope.header.ancestors?.genesis.substring(0, 16) ?? ""}</td>
    <td>{x.hash.substring(0, 16)}</td>
    <td>{x.envelope.header.height}</td>
    <td>{JSON.stringify(x.envelope.msg).substring(0, 20)}</td>
    <td><button onClick={() => console.log(x.envelope)}>log msg</button></td>
  </tr>);
  return <table>
    <thead>
      <tr>
        <th>Genesis</th>
        <th>Msg Hash</th>
        <th>Height</th>
        <th>Msg</th>
        <th>To Console</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
}

function ExpensiveMsgDB(props: { url: string }) {
  const [data, set_data] = React.useState({});
  const handle = async () => {
    const target = `${props.url}/expensive_db_snapshot`;
    console.log("Fetching...", target);
    try {
      const resp = await fetch(target);
      const js = await resp.json();
      set_data(js);
    }
    catch { }
  };
  const rows = Object.entries(data).map(([k, envelope]: [string, any]) => <tr key={k}>
    <td>{k.substring(0, 16)}</td>
    <td>{envelope.header.ancestors?.genesis.substring(0, 16) ?? ""}</td>
    <td>{envelope.header.height}</td>
    <td>{JSON.stringify(envelope.msg).substring(0, 20)}</td>
    <td><button onClick={() => console.log(envelope)}>log msg</button></td>
  </tr>);
  return <div>
    <button onClick={handle}>Refresh</button>
    <table>
      <thead>
        <tr>
          <th>Msg Hash</th>
          <th>Genesis</th>
          <th>Height</th>
          <th>Msg</th>
          <th>To Console</th>
        </tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
  </div>
}


function Peers(props: { peers: Array<{ service_url: string, port: string, fetch_from: boolean, push_to: boolean }> }) {

  const rows = props.peers.map(({ service_url, port, fetch_from, push_to }) => <tr key={`${service_url}:${port}`}>
    <td>{service_url}</td>
    <td>{port}</td>
    <td>{fetch_from ? "Enabled" : "Disabled"}</td>
    <td>{push_to ? "Enabled" : "Disabled"}</td>
  </tr>);
  return <table>
    <thead>
      <tr>
        <th>Host</th>
        <th>Port</th>
        <th>Fetch</th>
        <th>Push</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
}

function TaskSet(props: { tasks: Array<[string, number, "Fetch" | "Push"]> }) {
  const rows = props.tasks.map(([host, port, typ]) => <tr key={`${typ}://${host}:${port}`}>
    <td>{typ}</td>
    <td>{host}</td>
    <td>{port}</td>
  </tr>);
  return <table>
    <thead>
      <tr>
        <th>Task Type</th>
        <th>Host</th>
        <th>Port</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
}

function AddPeer(props: { root: string }) {

  const url = React.useRef<HTMLInputElement | null>(null);
  const port = React.useRef<HTMLInputElement | null>(null);
  function add_hidden(e: FormEvent) {
    e.preventDefault();
    if (!url.current || !port.current) return;
    fetch(`${props.root}/service`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          url: url.current?.value,
          port: port.current?.valueAsNumber
        })
      })
  }
  return <form onSubmit={add_hidden} >
    <input ref={url} name="url" type="text"></input>
    <input ref={port} name='port' type="number"></input>
    <button type="submit">Add Peer</button>
  </form>
}


export default App;
