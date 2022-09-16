import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';
import { DataGrid, GridActionsCellItem, GridColDef, GridColumns, GridRowParams, GridRowsProp, GridToolbarContainer } from '@mui/x-data-grid';
import { Cancel, Create, Newspaper, Report, Start } from '@mui/icons-material';
import { Button, Container } from '@mui/material';


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

const handle_new_msg = async (url: string, pk: string) => {
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

function Users(props: { users: Array<[string, string, boolean]>, url: string }) {

  const rows: GridRowsProp = props.users.map(([pubkey, nickname, has_private_key], id) => {
    return { id, pubkey, nickname, has_private_key }
  });
  const columns = [
    { field: 'pubkey', headerName: 'Public Key', minWidth: 300 },
    { field: 'nickname', headerName: 'Nickname', minWidth: 150 },
    { field: 'has_private_key', headerName: 'Known Secret Key', minWidth: 150 },
    {
      headerName: 'New Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Create></Create>} onClick={() =>
          handle_new_msg(props.url, rows[params.id as number].pubkey)} label="Create New Message" />,
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} />;

}
type Envelope = { header: { key: string, ancestors?: { genesis: string }, height: string }, msg: any };
function Tips(props: { tips: Array<{ envelope: Envelope, hash: string }> }) {
  const [view_flash, flash] = React.useState<null | string>(null);
  React.useEffect(() => {
    const t = setTimeout(() => view_flash && flash(null), 1000);
  }, [view_flash])
  const copy_on_click = (s: string) => {
    return (ev: React.MouseEvent<HTMLTableDataCellElement, MouseEvent>) => {
      flash(`Copied ${s}`);
      navigator.clipboard.writeText(s);
    }
  }

  const rows: GridRowsProp = props.tips.map((tip, id) => {
    const genesis = tip.envelope.header.ancestors?.genesis ?? tip.hash;
    const msg = JSON.stringify(tip.envelope.msg);
    const signing_key = tip.envelope.header.key;
    return { id, genesis, msg, signing_key, hash: tip.hash, height: tip.envelope.header.height }
  });
  const columns = [
    { field: 'genesis', headerName: 'Genesis', width: 300 },
    { field: 'signing_key', headerName: 'Public Key', width: 300 },
    { field: 'hash', headerName: 'Message Hash', width: 150 },
    { field: 'height', headerName: 'Height', width: 150 },
    { field: 'msg', headerName: 'Message', width: 150 },
    {
      headerName: 'Log Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Report></Report>} onClick={() =>
          console.log(["message"], JSON.parse(rows[params.id as number].msg))} label="Log Message" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} />;
}

function ExpensiveMsgDB(props: { url: string }) {
  const [data, set_data] = React.useState<Record<string, Envelope>>({});
  const [text, set_text] = React.useState("");
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

  const rows: GridRowsProp = Object.entries(data).map(([msg_hash, envelope]: [string, Envelope], id) => {
    const genesis = envelope.header.ancestors?.genesis ?? msg_hash;
    const msg = JSON.stringify(envelope.msg);
    const signing_key = envelope.header.key;
    return { id, genesis, msg, signing_key, hash: msg_hash, height: envelope.header.height }
  });
  const columns = [
    { field: 'genesis', headerName: 'Genesis', width: 300 },
    { field: 'signing_key', headerName: 'Public Key', width: 300 },
    { field: 'hash', headerName: 'Message Hash', width: 150 },
    { field: 'height', headerName: 'Height', width: 150 },
    { field: 'msg', headerName: 'Message', width: 150 },
    {
      headerName: 'Log Message',
      field: 'actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Report></Report>} onClick={() =>
          console.log(["message"], JSON.parse(rows[params.id as number].msg))} label="Log Message" />
      ]
    }
  ];
  function CustomToolbar() {
    return (
      <GridToolbarContainer>
        <h4>All Messages in DB</h4>
        <Button onClick={handle}> Refresh </Button>
      </GridToolbarContainer>
    );
  }

  return <DataGrid rows={rows} columns={columns} components={{ Toolbar: CustomToolbar }} />;
}


function Peers(props: { peers: Array<{ service_url: string, port: string, fetch_from: boolean, push_to: boolean }> }) {

  const rows: GridRowsProp = props.peers.map((peer, id) => {
    const row: typeof rows[number] = Object.fromEntries(Object.entries(peer));
    row.id = id;
    return row;
  });
  const columns = [
    { field: 'service_url', headerName: 'Host', minWidth: 100 },
    { field: 'port', headerName: 'Port', minWidth: 100 },
    { field: 'fetch_from', headerName: 'Fetch', minWidth: 50 },
    {
      headerName: 'Fetch Actions',
      field: 'fetch_pactions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Cancel></Cancel>} onClick={() =>
          console.log(["Peer"], "TODO")} label="Log Message" />,
        <GridActionsCellItem icon={<Start></Start>} onClick={() =>
          console.log(["Peer"], "TODO")} label="Log Message" />
      ]
    },
    { field: 'push_to', headerName: 'Push', width: 50 },
    {
      headerName: 'Push Actions',
      field: 'push_actions',
      type: 'actions',
      getActions: (params: GridRowParams) => [
        <GridActionsCellItem icon={<Cancel></Cancel>} onClick={() =>
          console.log(["Peer"], "TODO")} label="Log Message" />,
        <GridActionsCellItem icon={<Start></Start>} onClick={() =>
          console.log(["Peer"], "TODO")} label="Log Message" />
      ]
    }
  ];

  return <DataGrid rows={rows} columns={columns} />;
}

function TaskSet(props: { tasks: Array<[string, number, "Fetch" | "Push"]> }) {
  const rows: GridRowsProp = props.tasks.map(([host, port, typ], id) => {
    return { id, host, port, typ };
  });
  const columns = [
    { field: 'host', headerName: 'Host', minWidth: 100 },
    { field: 'port', headerName: 'Port', minWidth: 100 },
    { field: 'typ', headerName: 'Type', minWidth: 50 },
  ];

  return <DataGrid rows={rows} columns={columns} />;
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
