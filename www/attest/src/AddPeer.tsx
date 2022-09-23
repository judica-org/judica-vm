import { Button, Checkbox, FormControl, FormControlLabel, FormGroup, TextField } from '@mui/material';
import React, { FormEvent } from 'react';

export function AddPeer(props: { root: string | null; }) {

  console.log("Add Peer Render: ", props.root);

  const [url_text, set_url] = React.useState("");
  const [port_, set_port] = React.useState(-1);
  const url = React.useRef<HTMLInputElement | null>(null);
  const port = React.useRef<HTMLInputElement | null>(null);
  const fetch_from = React.useRef<HTMLButtonElement | null>(null);
  const push_to = React.useRef<HTMLButtonElement | null>(null);
  const allow_unsolicited_tips = React.useRef<HTMLButtonElement | null>(null);
  function add_hidden(e: FormEvent) {
    console.log("BUTTON PRESS", props.root)
    e.preventDefault();
    if (!url.current || !port.current || props.root === null)
      return;
    fetch(`${props.root}/service`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          url: url_text,
          port: port_,
          fetch_from: fetch_from.current?.value,
          push_to: push_to.current?.value,
          allow_unsolicited_tips: allow_unsolicited_tips.current?.value
        })
      });
  }
  return <FormControl  size="small" >
    <FormGroup row={true}>
      <FormGroup row={false}>
        <TextField hiddenLabel variant="filled" ref={url} name="url" type="text" label="Domain"
          size="small" onChange={(ev) => { set_url(ev.target.value) }} value={url_text}></TextField>
        <TextField hiddenLabel variant="filled" ref={port} name='port' type="number" label="Port" size="small"
          onChange={(ev) => { set_port(parseInt(ev.target.value)) }} value={port_}
        ></TextField>
      </FormGroup>
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={fetch_from} />} labelPlacement="bottom" label="Fetch" />
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={push_to} />} labelPlacement="bottom" label="Push" />
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={allow_unsolicited_tips} />} labelPlacement="bottom" label="Unsolicited" />

      <Button type="submit" size="small" onClick={add_hidden}>Add Peer</Button>
    </FormGroup>
  </FormControl>;
}
