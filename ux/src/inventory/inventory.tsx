import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, Typography, Divider, Button } from '@mui/material';
import { useEffect, useState } from 'react';
import { PowerPlant, UserInventory } from '../App';
import SaleListingForm from '../sale-listing/SaleListingForm';
import { EntityID } from '../Types/GameMove';
import { plant_type_color_map } from '../util';
import { UXUserInventory } from '../Types/Gameboard';


export const Inventory = ({ userInventory, currency }: { userInventory: UXUserInventory | null, currency: EntityID | null }) => {
  const [selected_plant_id, set_selected_plant_id] = useState<string | null>(null);


  return (
    <div>
      <div className='my-inventory-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            title={'Inventory'}
            subheader={'All assets owned'}
          />
          <CardContent >
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
                      {plant.coordinates}
                    </TableCell>
                    <TableCell align="right">{plant.hashrate}</TableCell>
                    <TableCell align="right">{plant.miners}</TableCell>
                    <TableCell align="right"><Button onClick={() => {
                      set_selected_plant_id(plant.id);
                    }}>List For Sale</Button></TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            {selected_plant_id && currency ? <SaleListingForm nft_id={selected_plant_id} currency={currency} /> : null}
            <Divider />
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