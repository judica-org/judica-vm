import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { PowerPlant } from '../App';
import FormModal from '../form-modal';
import { plant_type_color_map } from '../util';

export type UserPowerPlant = PowerPlant & {
  readonly hashrate: number | null;
}

export const MyPlants = () => {
const [plants, setPlants] = useState<UserPowerPlant[]|null>(null);

useEffect(() => {
  const unlisten_user_power_plants = appWindow.listen("user-power-plants", (ev) => {
    console.log(['user-power-plants'], ev);
    setPlants(JSON.parse(ev.payload as string) as UserPowerPlant[]);
  });

  return() => {
    (async () => {
      (await unlisten_user_power_plants)();
    })();
  }
}, [plants]);

  return (
    <div>
      <div className='my-plants-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            title={'My Power Plants'}
            subheader={'Will get plants by Owner here'}
          />
          <CardContent className={'content'}>
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
                {plants && plants.map((plant, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      {/* color code these in the future */}
                      {/* <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[plant.plant_type]}}/> */}
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
          </CardContent>
        </Card>
      </div>
    </div>
  )
};
export default MyPlants;

export const stuff = 'stuff';