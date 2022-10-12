// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
export type PeerInfo = {
  service_url: string,
  port: number,
  fetch_from: boolean,
  push_to: boolean,
  allow_unsolicited_tips: boolean,
  Error: undefined,
}

type TaskID = [[string, number], "Fetch" | "Push", boolean];
type Status = {
  peers: Array<PeerInfo>,
  tips: Array<{ envelope: Envelope, hash: string }>,
  peer_connections: Array<TaskID>,
  all_users: Array<[string, string, boolean]>,
  hidden_service_url: [string, number] | null;
  Error: undefined,
  IsNull: undefined,
}

function App() {
  const start = new URL(global.location.toString());
  const init = start.searchParams.get("service_url");
  const [url, set_url] = React.useState<null | string>(init);
  const [status, set_status] = React.useState<{ Error: undefined, IsNull: true } | Status | { Error: string, IsNull: true }>({ Error: undefined, IsNull: true });
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
        let js;
        try {
          const resp = await fetch(target);
          js = await resp.json();
        } catch (e) {
          console.log(e);
          set_status({ Error: "Error Connecting to service -- is it running?", IsNull: true })
          setTimeout(fetcher, 10000)
          return;
        }

        console.log(js);
        set_status(js);
        setTimeout(fetcher, 5000)
      }
      fetcher();
      return () => {
        cancel = true;
      }
    }
    , [url])
  const status_url = !status.IsNull && status.hidden_service_url ? `${status.hidden_service_url[0]}:${status.hidden_service_url[1]}` : null;
  return (
    <Box className="App">
      {peer}
      <AppBar position="static" color="secondary">
        <Toolbar variant="dense">
          <IconButton edge="start" color="inherit" aria-label="menu" sx={{ mr: 2 }}>
            <Menu />
          </IconButton>
          <Typography variant="body2" color="inherit" component="div" style={{ paddingRight: "10px" }}>
            {url || "YOU MUST SET A SERVICE URL"}
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
        {status.IsNull && status.Error}
        {!status.IsNull && <>
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
                  {!status.IsNull && <Peers peers={status?.peers ?? []} root={url} toolbar_component={peer}></Peers>}
                </div>
                <div style={{ gridArea: "tasks" }}>
                  {!status.IsNull && <TaskSet tasks={status.peer_connections}></TaskSet>}
                </div>
                <div style={{ gridArea: "tips" }}>
                  {!status.IsNull && <Tips tips={status.tips} set_genesis={(a) => { set_genesis(a); set_current_tab("commit_groups") }}></Tips>}
                </div>
                <div style={{ gridArea: "keys" }}>
                  {!status.IsNull && url && <Users users={status.all_users} url={url}></Users>}
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
        </>}
      </Container >
    </Box >
  );
}

export const handle_new_msg = async (safe: "SAFE" | "DANGER", url: string, pk: string) => {
  const message = safe === "DANGER" ?
    window.prompt("DANGER: You are about to destroy your chain.\n\nMessage must be [a, b] format?") :
    window.prompt("DANGER: Invalid message May Corrupt Your Chain.\n\nWhat message should we send?");
  if (message) {
    let js = JSON.parse(message);
    const c = window.confirm(`Are you sure? Pushing: \n ${JSON.stringify(message)}`);
    if (!c) return;
    const ret = fetch(`${url}/push_message_dangerous`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ msg: js, key: pk, equivocate: safe === "DANGER"})
    })
    console.log(await (await ret).json());
  }
};

export type Envelope = { header: { key: string, ancestors?: { genesis: string }, height: string }, msg: any };
export default App;