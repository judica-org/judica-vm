import FactoryIcon from '@mui/icons-material/Factory';
import { Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, Button, Divider } from '@mui/material';
import { useState, useEffect } from 'react';
import { PlantType } from '../App';
import PurchaseOfferForm from '../purchase-offer/PurchaseOfferForm';
import { EntityID } from '../Types/GameMove';
import { plant_type_color_map } from '../util';

export type NFTSale = {
  currency: EntityID,
  nft_id: EntityID,
  plant_type: PlantType
  price: number,
  seller: EntityID,
  transfer_count: number,
}

export const EnergyExchange = ({ listings }: { listings: NFTSale[] }) => {
  const [selected_listing, set_selected_listing] = useState<NFTSale | null>(null);
  const [currency, set_currency] = useState<string | null>(null);
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
                  <TableCell align="right">Plant ID</TableCell>
                  <TableCell align="right">Price ($Virtual BTC)</TableCell>
                  <TableCell align="right">Transfer Count</TableCell>
                  <TableCell align="right"></TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {listings && listings.map((listing, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      {listing.plant_type}
                      <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[listing.plant_type] }} />
                    </TableCell>
                    <TableCell component="th" scope="row">
                      {listing.seller}
                    </TableCell>
                    <TableCell align="right">{listing.nft_id}</TableCell>
                    <TableCell align="right">{listing.price}</TableCell>
                    <TableCell align="right">{listing.transfer_count}</TableCell>
                    <TableCell align="right"><Button onClick={() => {
                      set_selected_listing(listing);
                      set_currency(listing.currency);
                    }}>Purchase This Plant</Button></TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <Divider />
            {selected_listing && currency ? <PurchaseOfferForm nft_id={selected_listing.nft_id} currency={currency} listing_price={selected_listing.price} /> : null}
          </CardContent>
        </Card>
      </div>
    </div>
  )
};
export default EnergyExchange;