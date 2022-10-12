// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import FactoryIcon from '@mui/icons-material/Factory';
import { Table, TableHead, TableRow, TableCell, TableBody, Typography, Divider, Button, Paper, FormControl, FormLabel, Select, MenuItem } from '@mui/material';
import { FormEvent, useEffect, useState } from 'react';
import SaleListingForm from '../sale-listing/SaleListingForm';
import { EntityID } from '../Types/GameMove';
import { plant_type_color_map } from '../util';
import { UXPlantData, UXUserInventory } from '../Types/Gameboard';
import { MoveHashboards } from '../move-hashboards/MoveHashboards';
import { COORDINATE_PRECISION } from '../mint-power-plant/MintingForm';
import { tauri_host } from '../tauri_host';

const NO_KEY = "No Key Available";
export type InventoryProps = {
  player_key_map: { [k: string]: string },
  signing_key: string | null,
  currency: EntityID | null,
  hashboard_pointer: EntityID | null
}

export const Inventory = ({ player_key_map, signing_key, currency, hashboard_pointer }: InventoryProps) => {
  const [userInventory, setUserInventory] = useState<UXUserInventory | null>(null);
  const [selected_plant_id_sale, set_selected_plant_id_sale] = useState<string | null>(null);
  const [selected_plant_hashboards, set_selected_plant_hashboards] = useState<UXPlantData | null>(null);
  const [user_hashboards, set_user_hashboards] = useState<number>(0);
  const [selected_key, set_selected_key] = useState<string>(signing_key ?? "No Key Available");
  const [total_hashrate, set_total_hashrate] = useState<number>(0);

  const getInventory = async (key: string) => {
    try {
      const inventory = await tauri_host.get_inventory_by_key(selected_key);
      setUserInventory(inventory);
      const hashboards = inventory.user_token_balances.find(([name, _number]) => name === "ASIC Gen 1") ?? ["ASIC Gen 1", 0];
      set_user_hashboards(hashboards[1]);
      const total_hashrate = Object.values(inventory.user_power_plants).reduce((acc, plant) => {
        return acc += plant.hashrate;
      }, 0)
      set_total_hashrate(total_hashrate);
    } catch (e) {
      console.warn(e);
    }
  }

  useEffect(() => {
    if (selected_key != NO_KEY) {
      getInventory(selected_key)
    }
  }, [signing_key])

  const handle_submit = async (ev: FormEvent<HTMLButtonElement>) => {
    ev.preventDefault();
    console.log(["selected-key"], selected_key);
    // redundant but more clear to check both
    await getInventory(selected_key);
  };

  let player_options = Object.entries(player_key_map).map(([key, id]) => {
    return <MenuItem value={key} selected={key === selected_key} key={key}>{id}</MenuItem>;
  })
  const disable_owner_actions = selected_key != signing_key;

  return (
    <div>
      <div className='my-inventory-container'>
        <Paper>
          <div className='InventoryTitle'>
            <Typography variant='h4'>Inventory</Typography>
            <Typography variant='body1'>All Assets Owned</Typography>
          </div>
          <Divider />
          <FormControl>
            <FormLabel>Player Id:</FormLabel>
            <Select label="Player Key"
              value={selected_key}
              onChange={(ev) => set_selected_key(ev.target.value as string)}
            >
              {player_options}
            </Select>
            <Button variant="contained" type="submit" onClick={handle_submit}>View Inventory</Button>
          </FormControl>
          <Typography variant='h6'>Power Plants</Typography>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Plant Type</TableCell>
                <TableCell>Location</TableCell>
                <TableCell align="right">Hashrate</TableCell>
                <TableCell align="right">Miners Allocated</TableCell>
                <TableCell align="right">Owner Actions</TableCell>

              </TableRow>
            </TableHead>
            <TableBody>
              {Object.entries(userInventory?.user_power_plants ?? {}).map(([ptr, plant]) => (
                <TableRow key={`plant-${ptr}`}>
                  <TableCell>
                    <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[plant.plant_type] }} /><p>{plant.plant_type}</p>
                  </TableCell>
                  <TableCell component="th" scope="row">
                    {`${plant.coordinates[0] / COORDINATE_PRECISION}, ${plant.coordinates[1] / COORDINATE_PRECISION}`}
                  </TableCell>
                  <TableCell align="right">{plant.hashrate}</TableCell>
                  <TableCell align="right">{plant.miners}</TableCell>
                  <TableCell align="right">
                    <Button disabled={disable_owner_actions} onClick={() => {
                      set_selected_plant_id_sale(plant.id);
                    }}>List For Sale</Button>
                    <Button disabled={disable_owner_actions} onClick={() => {
                      set_selected_plant_hashboards(plant);
                    }}>Move Hashboards</Button>
                  </TableCell>
                </TableRow>
              ))}
              <TableRow>
                <TableCell>Total Hashrate</TableCell>
                <TableCell></TableCell>
                <TableCell align="right">{total_hashrate}</TableCell>
                <TableCell align="right"></TableCell>
                <TableCell align="right"></TableCell>
              </TableRow>
            </TableBody>
          </Table>
          {selected_plant_id_sale && currency ? <SaleListingForm nft_id={selected_plant_id_sale} currency={currency} /> : null}
          {selected_plant_hashboards && hashboard_pointer ? <MoveHashboards action={'ADD'} plant={selected_plant_hashboards} user_hashboards={user_hashboards} hashboard_pointer={hashboard_pointer}></MoveHashboards> : null}
          <Divider />
          <Typography variant='h6'>Tokens</Typography>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Asset</TableCell>
                <TableCell align="right">Quantity</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {userInventory?.user_token_balances && userInventory?.user_token_balances.map((token, index) => (
                <TableRow key={index}>
                  <TableCell component="th" scope="row">
                    {token[0]}
                  </TableCell>
                  <TableCell align="right">{token[1]}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Paper>
      </div>
    </div>
  )
};
export default Inventory;

export const stuff = 'stuff';