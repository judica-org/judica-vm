import React, { FormEvent } from 'react';

export function AddPeer(props: { root: string; }) {


  const url = React.useRef<HTMLInputElement | null>(null);
  const port = React.useRef<HTMLInputElement | null>(null);
  function add_hidden(e: FormEvent) {
    e.preventDefault();
    if (!url.current || !port.current)
      return;
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
      });
  }
  return <form onSubmit={add_hidden}>
    <input ref={url} name="url" type="text"></input>
    <input ref={port} name='port' type="number"></input>
    <button type="submit">Add Peer</button>
  </form>;
}
