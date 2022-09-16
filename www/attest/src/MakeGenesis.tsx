import React from 'react';

export function MakeGenesis(props: { url: String; }) {
  const handle = async () => {
    const new_genesis = window.prompt("New Genesis Named?");
    if (new_genesis) {

      const ret = fetch(`${props.url}/make_genesis`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(new_genesis)
      });
      console.log(await (await ret).json());
    }
  };
  return <button onClick={() => handle()}>New Genesis</button>;
}
