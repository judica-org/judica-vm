import { Button, FormControl, FormLabel, TextField } from '@mui/material';
import React from 'react';
import { tauri_host } from '../tauri_host';
export interface SwitchToHostProps { game_host_service: { url: string, port: number } | null }
export function SwitchToHost(props: SwitchToHostProps) {
  const [port, set_port] = React.useState<number | null>(null);
  const [url, set_url] = React.useState<string | null>(null);


  const handle_submit = (ev: React.FormEvent<HTMLButtonElement>): void => {
    ev.preventDefault();
    // prefix allowed to be null
    port && url && tauri_host.set_game_host({ url, port });
  };
  return <div>
    <FormControl>
      <FormLabel>Game Host:</FormLabel>
      <FormLabel sx={{ wordBreak: "break-word" }} component="code">
        {props.game_host_service &&
          `${props.game_host_service.url}:${props.game_host_service.port}`
        }</FormLabel>
      <TextField label="Host Name" required={false} onChange={(ev) => set_url(ev.target.value)}></TextField>
      <TextField label="Port" required={true} type="number" onChange={(ev) => set_port(parseInt(ev.target.value))}></TextField>
      <Button variant="contained" type="submit"
        onClick={handle_submit}
      >Switch Service </Button>
    </FormControl>
  </div>;
}
