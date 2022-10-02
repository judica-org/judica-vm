import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody } from '@mui/material';
import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect } from 'react';
import { PlantType } from '../App';
import FormModal from '../form-modal/FormModal';
import { EntityID } from '../Types/GameMove';
import { plant_type_color_map } from '../util';

export type NFTSale = {
  currency: any,
  nft_id: EntityID,
  plant_type: PlantType
  price: number,
  seller: EntityID,
  transfer_count: number,
}

const stub_listings: NFTSale[] = [{
  currency: 'donuts',
  nft_id: "13134",
  plant_type: 'Flare',
  price: 937,
  seller: "95720486",
  transfer_count: 2,
}, {
  currency: 'cookies',
  nft_id: "26783",
  plant_type: 'Solar',
  price: 424,
  seller: "3058572037",
  transfer_count: 1,
}]

export const EnergyExchange = ({listings}:{listings:NFTSale[]}) => {
  return (
    <div>
      <div className='energy-exchange-container'>
        <Card className={'card'}>
          <CardHeader
            className={'root'}
            title={'Energy Exchange'}
            subheader={'Power Plants For Sale'}
          />
          <CardContent className={'content'}>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Plant Type</TableCell>
                  <TableCell>Seller</TableCell>
                  <TableCell align="right">Price ($)</TableCell>
                  <TableCell align="right">Currency (token)</TableCell>
                  <TableCell align="right">Transfer Count</TableCell>
                  <TableCell align="right"></TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {listings && listings.map((listing, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      {/* color code these in the future */}
                      <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[listing.plant_type] }} />
                    </TableCell>
                    <TableCell component="th" scope="row">
                      {listing.seller}
                    </TableCell>
                    <TableCell align="right">{listing.price}</TableCell>
                    <TableCell align="right">{listing.currency}</TableCell>
                    <TableCell align="right">{listing.transfer_count}</TableCell>
                    <TableCell align="right"><FormModal action="Purchase Plant" title={"Purchase Plant"} nft_id={listing.nft_id}  /></TableCell>
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
export default EnergyExchange;