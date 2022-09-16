import { Button, Checkbox, FormControl, FormControlLabel, FormGroup, TextField } from '@mui/material';
import React, { FormEvent } from 'react';

export function AddPeer(props: { root: string; }) {


  const url = React.useRef<HTMLInputElement | null>(null);
  const port = React.useRef<HTMLInputElement | null>(null);
  const fetch_from = React.useRef<HTMLButtonElement | null>(null);
  const push_to = React.useRef<HTMLButtonElement | null>(null);
  const allow_unsolicited_tips = React.useRef<HTMLButtonElement | null>(null);
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
          port: port.current?.valueAsNumber,
          fetch_from: fetch_from.current?.value,
          push_to: push_to.current?.value,
          allow_unsolicited_tips: allow_unsolicited_tips.current?.value
        })
      });
  }
  return <FormControl onSubmit={add_hidden} size="small">
    <FormGroup row={true}>
      <FormGroup row={false}>
        <TextField hiddenLabel variant="filled" ref={url} name="url" type="text" label="Domain" size="small"></TextField>
        <TextField hiddenLabel variant="filled" ref={port} name='port' type="number" label="Port" size="small"></TextField>
      </FormGroup>
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={fetch_from} />} labelPlacement="bottom" label="Fetch" />
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={push_to} />} labelPlacement="bottom" label="Push" />
      <FormControlLabel control={<Checkbox size="small" defaultChecked ref={allow_unsolicited_tips} />} labelPlacement="bottom" label="Unsolicited" />

      <Button type="submit" size="small">Add Peer</Button>
    </FormGroup>
  </FormControl>;
}
