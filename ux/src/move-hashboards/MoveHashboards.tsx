import { Card, CardHeader, CardContent, Typography, FormControl, TextField, Button } from "@mui/material";
import { useState } from "react";
import { UserPowerPlant } from "../App";
import { tauri_host } from "../tauri_host";
import { MoveHashboardsActions } from "../util";

export const MoveHashboards = ({ action: initial_action, plant, user_hashboards, hashboard_pointer }:
  { readonly action: MoveHashboardsActions, plant: UserPowerPlant, user_hashboards: number, hashboard_pointer: string }) => {
  const [action, set_action] = useState<MoveHashboardsActions>(initial_action);
  const [hashboard_qty, set_hashboard_qty] = useState<number>(0);
  const switch_action = () => {
    switch (action) {
      case "ADD":
        return ("REMOVE")
      case "REMOVE":
        return ("ADD")
    };
  }

  const handle_click = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {
    // need the hashboard token id
    ev.preventDefault();
    if (hashboard_qty && action === "ADD")
      tauri_host.make_move_inner({ send_tokens: { amount: hashboard_qty, currency: (hashboard_pointer), to: plant.id } });
    if (hashboard_qty && action === "REMOVE")
      tauri_host.make_move_inner({ remove_tokens: { nft_id: plant.id, amount: hashboard_qty, currency: (hashboard_pointer) } });
  };

  return <Card>
    <CardHeader
      title={action}
      subheader={`${action === "ADD" ? user_hashboards : plant.miners} Hashboards Available`}
    >
    </CardHeader>
    <CardContent>
      <Typography variant="h6">
        {action} hashboards {action === "ADD" ? 'to' : 'from'} PlantId: {plant.id}
      </Typography>
      <div className='MoveForm' >
        <FormControl >
          <TextField label={"Hashboards to move:"} type="number" value={hashboard_qty} onChange={(ev) => { set_hashboard_qty(parseInt(ev.target.value)) }}></TextField>
          <Button type="submit" onClick={handle_click}>{action}</Button>
          <Button onClick={() => set_action(switch_action())}>Switch To {switch_action()} </Button>
        </FormControl>
      </div>
    </CardContent>
  </Card>;
};