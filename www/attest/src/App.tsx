import React from 'react';
import logo from './logo.svg';
import './App.css';
import { GridColDef, GridColumns } from '@mui/x-data-grid';
import { Menu, Newspaper } from '@mui/icons-material';
import { AppBar, Box, Container, IconButton, Toolbar, Typography } from '@mui/material';
import { AddPeer } from './AddPeer';
import { TaskSet } from './TaskSet';
import { Peers } from './Peers';
import { ExpensiveMsgDB } from './ExpensiveMsgDB';
import { Tips } from './Tips';
import { Users } from './Users';
import { MakeGenesis } from './MakeGenesis';
import { ChangeService } from './ChangeService';


function App() {
  const start = new URL(global.location.toString());
  const init = start.searchParams.get("service_url");
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
  return (
    <Box className="App">
      <AppBar position="static" color="secondary">
        <Toolbar variant="dense">
          <IconButton edge="start" color="inherit" aria-label="menu" sx={{ mr: 2 }}>
            <Menu />
          </IconButton>
          <Typography variant="body2" color="inherit" component="div" style={{ paddingRight: "10px" }}>
            {url}
          </Typography>
          <ChangeService set_url={set_url} ></ChangeService>
          <Typography variant="body2" color="inherit" component="div" style={{ paddingLeft: "10px" }}>
            Tor: {status && status.hidden_service_url && status.hidden_service_url[0]}:{status && status.hidden_service_url[1]}
          </Typography>
        </Toolbar>
      </AppBar>
      <Container maxWidth={"lg"} className="Main">

        <div className="TableGrid">

          <div style={{ gridArea: "peers" }}>
            {status && url && <Peers peers={status.peers} root={url}></Peers>}
          </div>
          <div style={{ gridArea: "tasks" }}>
            {status && <TaskSet tasks={status.peer_connections}></TaskSet>}
          </div>
          <div style={{ gridArea: "tips" }}>
            {status && <Tips tips={status.tips}></Tips>}
          </div>
          <div style={{ gridArea: "keys" }}>
            {status && url && <Users users={status.all_users} url={url}></Users>}
          </div>
          <div style={{ gridArea: "all-msgs" }}>
            {url ? <ExpensiveMsgDB url={url}></ExpensiveMsgDB> : <div></div>}
          </div>
        </div>
      </Container>
    </Box>
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