import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, Typography } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { PowerPlant } from '../App';
import FormModal from '../form-modal';
import { plant_type_color_map } from '../util';

export type UserPowerPlant = PowerPlant & {
  readonly hashrate: number | null;
}

export type UserInventory = {
  user_power_plants: UserPowerPlant[]
  user_token_balances: (string | number)[][]
}

const user_inventory_stub = {
  user_power_plants: [],
  user_token_balances: [['Bitcoin', 400], ['Steel', 398], ['Silicon', 201], ['Concrete', 267]]
}

const prices = ["N/A", 5, 66, 24];

export const Inventory = () => {
  const [userInventory, setUserInventory] = useState<UserInventory | null>(null);

  useEffect(() => {
    setUserInventory(user_inventory_stub);
    // const unlisten_user_inventory = appWindow.listen("user-inventory", (ev) => {
    //   console.log(['user-inventory'], ev);
    //   setUserInventory(ev.payload as UserInventory);
    // });

    // return () => {
    //   (async () => {
    //     (await unlisten_user_inventory)();
    //   })();
    // }
  }, [userInventory]);

  return (
    <div>

      {/* <Typography variant='h4'>Power Plants</Typography>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Plant Type</TableCell>
                  <TableCell>Location</TableCell>
                  <TableCell align="right">Hashrate</TableCell>
                  <TableCell align="right">Miners Allocated</TableCell>
                  <TableCell align="right">More Actions</TableCell>

                </TableRow>
              </TableHead>
              <TableBody>
                {userInventory?.user_power_plants && userInventory?.user_power_plants.map((plant, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[plant.plant_type] }} /><p>{plant.plant_type}</p>
                    </TableCell>
                    <TableCell component="th" scope="row">
                      {plant.coordinates}
                    </TableCell>
                    <TableCell align="right">{plant.hashrate}</TableCell>
                    <TableCell align="right">{plant.has_miners ? 'yes' : 'no'}</TableCell>
                    <TableCell align="right"><FormModal title={'Sell Plant'} currency={'Bitcoin'} nft_id={plant.id} /><div>Plant Detail</div></TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table> */}
      <Table size="small" sx={{ borderTop: 1, borderBottom: 1 }}>
        <TableHead>
          <TableRow key="titles">
            <TableCell variant="head">
              Inventory
            </TableCell>
            {/* make row headers */}
            {userInventory?.user_token_balances && userInventory?.user_token_balances.map((token, index) => (
              <TableCell variant="head">
                {token[0]}
              </TableCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          <TableRow key="values">
            <TableCell>
              Qty Owned
            </TableCell>
            {/* make cells */}
            {userInventory?.user_token_balances && userInventory?.user_token_balances.map((token, index) => (
              <TableCell component="td" scope="row">
                {token[1]}
              </TableCell>
            ))}
          </TableRow>
          <TableRow key="prices">
            <TableCell>
              Price in BTC
            </TableCell>
            {
              prices && prices.map((val, index) => (
                <TableCell component="td" scope="row">
                  {val}
                </TableCell>
              ))
            }
          </TableRow>
        </TableBody>
      </Table>

    </div>
  )
};
export default Inventory;

export const stuff = 'stuff';