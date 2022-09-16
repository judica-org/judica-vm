import { Button } from '@mui/material';
import React from 'react';

export function MakeGenesis(props: { url: String; }) {
  const handle = async () => {
    const new_genesis = window.prompt("New Genesis Named?");
    if (!new_genesis) return;
    const obj = window.prompt("First Message as JSON? (warning, must be valid for the application)");
    if (!new_genesis) return;
    if (!obj) return;

    const ret = fetch(`${props.url}/make_genesis`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        nickname: new_genesis,
        msg: JSON.parse(obj)
      })
    });
    console.log(await (await ret).json());
  };
  return <Button size="small" onClick={() => handle()}>New Genesis</Button>;
}
