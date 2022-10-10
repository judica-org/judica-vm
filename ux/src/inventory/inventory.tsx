import FactoryIcon from '@mui/icons-material/Factory';
import { Table, TableHead, TableRow, TableCell, TableBody, Typography, Divider, Button, Paper } from '@mui/material';
import { useEffect, useState } from 'react';
import SaleListingForm from '../sale-listing/SaleListingForm';
import { EntityID } from '../Types/GameMove';
import { plant_type_color_map } from '../util';
import { UXPlantData, UXUserInventory } from '../Types/Gameboard';
import { MoveHashboards } from '../move-hashboards/MoveHashboards';
import { COORDINATE_PRECISION } from '../mint-power-plant/MintingForm';


export const Inventory = ({ userInventory, currency, hashboard_pointer }: { userInventory: UXUserInventory | null, currency: EntityID | null, hashboard_pointer: EntityID | null }) => {
  const [selected_plant_id_sale, set_selected_plant_id_sale] = useState<string | null>(null);
  const [selected_plant_hashboards, set_selected_plant_hashboards] = useState<UXPlantData | null>(null);
  const [user_hashboards, set_user_hashboards] = useState<number>(0);

  useEffect(() => {
    if (userInventory) {
      const hashboards = userInventory.user_token_balances.find(([name, _number]) => name === "ASIC Gen 1") ?? ["ASIC Gen 1", 0];
      set_user_hashboards(hashboards[1]);
    }
  }, [userInventory])

  return (
    <div>
      <div className='my-inventory-container'>
        <Paper>
          <div className='InventoryTitle'>

            <Typography variant='h4'>Inventory</Typography>
            <Typography variant='body1'>All Assets Owned</Typography>
          </div>
          <Divider />

          <Typography variant='h6'>Power Plants</Typography>
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
                  <TableCell align="right">
                    <Button onClick={() => {
                      set_selected_plant_id_sale(plant.id);
                    }}>List For Sale</Button>
                    <Button onClick={() => {
                      set_selected_plant_hashboards(plant);
                    }}>Move Hashboards</Button>
                  </TableCell>
                </TableRow>
              ))}
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