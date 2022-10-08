import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, Typography } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { UserInventory } from '../App';
import FormModal from '../form-modal/FormModal';
import { COORDINATE_PRECISION } from '../mint-power-plant/MintingForm';
import { UXUserInventory } from '../Types/Gameboard';
import { plant_type_color_map } from '../util';


export const Inventory = ({ userInventory }: { userInventory: UXUserInventory | null }) => {

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
                    <TableCell align="right"><FormModal action={"Sell Plant"} title={'Sell Plant'}  nft_id={plant.id} /><div>Plant Detail</div></TableCell>
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