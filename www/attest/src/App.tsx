import React from 'react';
import logo from './logo.svg';
import './App.css';
import { GridColDef, GridColumns, GridToolbarContainer } from '@mui/x-data-grid';
import { CopyAll, Menu, Newspaper } from '@mui/icons-material';
import { AppBar, Box, Button, Container, IconButton, Tab, Tabs, Toolbar, Typography } from '@mui/material';
import { AddPeer } from './AddPeer';
import { TaskSet } from './TaskSet';
import { Peers } from './Peers';
import { ExpensiveMsgDB } from './ExpensiveMsgDB';
import { Tips } from './Tips';
import { Users } from './Users';
import { MakeGenesis } from './MakeGenesis';
import { ChangeService } from './ChangeService';
import { ChainCommitGroups } from './ChainCommitGroups';

function Panel({ my_id, current_tab, children }: React.PropsWithChildren<{ my_id: string, current_tab: string }>) {
  return <div hidden={my_id !== current_tab}>
    {my_id === current_tab && children}
  </div>
}

function App() {
  const start = new URL(global.location.toString());
  const init = start.searchParams.get("service_url");
  const [url, set_url] = React.useState<null | string>(init);
  const [status, set_status] = React.useState<null | any>(null);
  const [genesis, set_genesis] = React.useState<null | string>(null);
  const [current_tab, set_current_tab] = React.useState<"main" | "commit_groups">("main");
  const peer = React.useMemo(() =>
    <AddPeer root={url}></AddPeer>
    , [url]);
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
  const status_url = status && status.hidden_service_url ? `${status.hidden_service_url[0]}:${status.hidden_service_url[1]}` : null;
  return (
    <Box className="App">
      {peer}
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
            {status_url === null ? "Tor Disabled" :
              <>
                Tor: <code>{status_url}</code>
                <IconButton onClick={() => { window.navigator.clipboard.writeText(status_url) }}><CopyAll></CopyAll></IconButton>
              </>
            }
          </Typography>
        </Toolbar>
      </AppBar>
      <Container maxWidth={"lg"} className="Main">


        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>

          <Tabs onChange={(ev, t) => set_current_tab(t)} value={current_tab}>
            <Tab value="main" label="Main"></Tab>
            <Tab value="commit_groups" label="Commit Groups"></Tab>
          </Tabs>
        </Box>
        <Box>

          <Panel current_tab={current_tab} my_id={"main"}>

            <div className="TableGrid">

              <div style={{ gridArea: "peers" }}>
                <Peers peers={status?.peers ?? []} root={url} toolbar_component={peer}></Peers>
              </div>
              <div style={{ gridArea: "tasks" }}>
                {status && <TaskSet tasks={status.peer_connections}></TaskSet>}
              </div>
              <div style={{ gridArea: "tips" }}>
                {status && <Tips tips={status.tips} set_genesis={(a) => { set_genesis(a); set_current_tab("commit_groups") }}></Tips>}
              </div>
              <div style={{ gridArea: "keys" }}>
                {status && url && <Users users={status.all_users} url={url}></Users>}
              </div>
              <div style={{ gridArea: "all-msgs" }}>
                {url ? <ExpensiveMsgDB url={url}></ExpensiveMsgDB> : <div></div>}
              </div>

            </div>
          </Panel>

          <Panel current_tab={current_tab} my_id={"commit_groups"}>
            <div className="TableGridChainCommit">

              <div style={{ gridArea: "commit-groups" }}>
                {url && genesis ? <ChainCommitGroups url={url} genesis={genesis}></ChainCommitGroups> : <div></div>}
              </div>
            </div>
          </Panel>
        </Box>
      </Container >
    </Box >
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