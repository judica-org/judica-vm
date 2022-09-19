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
  user_token_balances: [string, number][]
}

export const Inventory = () => {
  const [userInventory, setUserInventory] = useState<UserInventory | null>(null);

  useEffect(() => {
    const unlisten_user_inventory = appWindow.listen("user-inventory", (ev) => {
      console.log(['user-inventory'], ev);
      setUserInventory(ev.payload as UserInventory);
    });

    return () => {
      (async () => {
        (await unlisten_user_inventory)();
      })();
    }
  }, [userInventory]);

  return (
    <div>
      <div className='my-inventory-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            title={'Inventory'}
            subheader={'All assets owned'}
          />
          <CardContent className={'content'}>
            <Typography variant='h4'>Power Plants</Typography>
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
            </Table>
            <Typography variant='h4'>Tokens</Typography>
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
          </CardContent>
        </Card>
      </div>
    </div>
  )
};
export default Inventory;

export const stuff = 'stuff';