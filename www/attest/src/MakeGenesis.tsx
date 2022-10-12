// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
export function MakeGenesisImported(props: { url: String; }) {
  const handle = async () => {
    const new_genesis = window.prompt("New Genesis Named?");
    if (!new_genesis) return;
    const danger_extended_private_key = window.prompt("New Genesis From EPK?");
    if (!danger_extended_private_key) return;
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
        msg: JSON.parse(obj),
        danger_extended_private_key
      })
    });
    const r = await ret;
    console.log(await r.body);
  };
  return <Button size="small" onClick={() => handle()}>New Genesis (Imported EPK)</Button>;
}

