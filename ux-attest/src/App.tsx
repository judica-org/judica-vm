import React, { FormEvent } from 'react';
import logo from './logo.svg';
import './App.css';

function App() {
  const service = React.useRef<HTMLInputElement | null>(null);
  const [url, set_url] = React.useState<null | string>(null);
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
      {url && <div>{JSON.stringify(status)}</div>}
    </div>
  );
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
