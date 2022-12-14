// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import ConstructionIcon from '@mui/icons-material/Construction';
import { Typography, Card, CardHeader, CardContent, Table, TableHead, TableRow, TableCell, TableBody, Button, Divider } from '@mui/material';
import { useState, useEffect } from 'react';
import { MaterialPriceDisplay } from '../App';
import PurchaseMaterialForm from '../purchase-material/PurchaseMaterialForm';
import { RawMaterialsActions } from '../util';


export const RawMaterialsMarket = ({ materials }: { materials: MaterialPriceDisplay[] }) => {
  const [selected_material, set_selected_material] = useState<MaterialPriceDisplay | null>(null);
  const [action, set_action] = useState<RawMaterialsActions | null>(null);

  return (
    <div>
      <div className='materials-market-container'>
        <Card className={'card'}>
          <CardHeader
            className={"MaterialsMarketTitle"}
            title={'Marketplace'}
            subheader={'Buy and sell resources to build powerplants and mining operations'}
          />
          <CardContent className={'content'}>
            <Divider />
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Material</TableCell>
                  <TableCell align="right">Price Per Unit</TableCell>
                  <TableCell align="right">Currency (token)</TableCell>
                  <TableCell align="right">Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {materials && materials.map((material, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <ConstructionIcon className='sale-factory-icon' fontSize={'medium'} />
                      <Typography>{material.display_asset}</Typography>
                    </TableCell>
                    <TableCell align="right">1 to {Math.round(material.price_a_b_b_a[1])}</TableCell>
                    <TableCell align="right">{material.asset_b} / {material.asset_a}</TableCell>
                    <TableCell align="right">
                      <Button onClick={() => {
                        set_action("BUY");
                        set_selected_material(material);
                      }}>Trade</Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <Divider />
            {(action && selected_material) && <PurchaseMaterialForm action={action} market={selected_material} />}
          </CardContent>
        </Card>
      </div>
    </div >
  )
};
export default RawMaterialsMarket;