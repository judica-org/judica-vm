import { Button, Checkbox, FormControl, FormControlLabel, FormGroup, TextField } from '@mui/material';
import React, { FormEvent } from 'react';

function add_hidden(e: FormEvent, url_text: string, root: string|null) {
  console.log("BUTTON PRESS", root)
  e.preventDefault();
  const [url, port_str] = url_text.split(":", 2);
  const port = parseInt(port_str);

  if (root === null)
    return;
  fetch(`${root}/service`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        url: url,
        port: port,
        fetch_from: false,
        push_to: false,
        allow_unsolicited_tips: false,
      })
    });
}
export function AddPeer(props: { root: string | null; }) {

  console.log("Add Peer Render: ", props.root);
  const [url_text, set_url] = React.useState("");
  return <FormControl size="small" >
    <FormGroup row={true}>
      <FormGroup row={false}>
        <TextField hiddenLabel variant="filled" name="url" type="text" label="Domain:Port"
          size="small" onChange={(ev) => { set_url(ev.target.value) }} value={url_text}></TextField>
      </FormGroup>
      <Button type="submit" size="small" onClick={(ev) => add_hidden(ev, url_text, props.root)}>Add Peer</Button>
    </FormGroup>
  </FormControl>;
}
