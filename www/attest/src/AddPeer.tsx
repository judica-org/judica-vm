import { Button, FormControl, FormGroup, TextField } from '@mui/material';
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
  return <FormControl onSubmit={add_hidden} size="small">
    <FormGroup row={true}>
      <TextField ref={url} name="url" type="text" label="Domain" size="small"></TextField>
      <TextField ref={port} name='port' type="number" label="Port" size="small"></TextField>
      <Button type="submit">Add Peer</Button>
    </FormGroup>
  </FormControl>;
}
