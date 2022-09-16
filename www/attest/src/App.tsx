import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';
import { GridColDef, GridColumns } from '@mui/x-data-grid';
import { Newspaper } from '@mui/icons-material';
import { Container } from '@mui/material';
import { AddPeer } from './AddPeer';
import { TaskSet } from './TaskSet';
import { Peers } from './Peers';
import { ExpensiveMsgDB } from './ExpensiveMsgDB';
import { Tips } from './Tips';
import { Users } from './Users';
import { MakeGenesis } from './MakeGenesis';


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
      <Container maxWidth={"lg"}>

        <div className="TableGrid">
          <div style={{ gridArea: "head" }}>
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
              <h6>{status.hidden_service_url[0]}:{status.hidden_service_url[1]} </h6>
            </div>}
          </div>

          <div style={{ gridArea: "a" }}>
            <h4>Peers</h4>
            {status && <Peers peers={status.peers}></Peers>}
          </div>
          <div style={{ gridArea: "b" }}>
            <h4>Tasks</h4>
            {status && <TaskSet tasks={status.peer_connections}></TaskSet>}
          </div>
          <div style={{ gridArea: "c" }}>
            <h4>Tips</h4>
            {status && <Tips tips={status.tips}></Tips>}
          </div>
          <div style={{ gridArea: "d" }}>
            <h4>Key Chain</h4>
            {status && url && <Users users={status.all_users} url={url}></Users>}
          </div>
          <div style={{ gridArea: "e" }}>
            <h4>DB Snapshot</h4>
            {url ? <ExpensiveMsgDB url={url}></ExpensiveMsgDB> : <div></div>}
          </div>
        </div>
      </Container>
    </div>
  );
}

export const handle_new_msg = async (url: string, pk: string) => {
  const message = window.prompt("DANGER: Invalid message May Corrupt Your Chain.\n\nWhat message should we send?");
  if (message) {
    let js = JSON.parse(message);
    const c = window.confirm(`Are you sure? Pushing: \n ${JSON.stringify(message)}`);
    if (!c) return;
    const ret = fetch(`${url}/push_message_dangerous`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ msg: js, key: pk })
    })
    console.log(await (await ret).json());
  }
};

export type Envelope = { header: { key: string, ancestors?: { genesis: string }, height: string }, msg: any };
export default App;
