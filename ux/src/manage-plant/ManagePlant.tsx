// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Card, CardHeader, CardContent, Typography, Table, TableBody, TableCell, TableHead, TableRow, Divider } from "@mui/material";
import FactoryIcon from '@mui/icons-material/Factory';
import { useEffect, useState } from "react"
import { UserPowerPlant } from "../App";
import { plant_type_color_map } from "../util";
import { MoveHashboards } from "../move-hashboards/MoveHashboards";
import { EntityID } from "../Types/GameMove";
import SaleListingForm from "../sale-listing/SaleListingForm";
import PurchaseOfferForm from "../purchase-offer/PurchaseOfferForm";
import { UXUserInventory } from "../Types/Gameboard";
import { COORDINATE_PRECISION } from "../mint-power-plant/MintingForm";

export type PlantLabel = { readonly id: EntityID, readonly owner: EntityID, readonly watts: string, readonly for_sale: boolean };

export const ManagePlant = ({ asic_token_id, bitcoin_token_id, selected_plant, power_plants, user_inventory }:
  { asic_token_id: string | null, bitcoin_token_id: string | null, selected_plant: EntityID | null, power_plants: UserPowerPlant[] | null, user_inventory: UXUserInventory | null }) => {

  // extracted from user_inventory
  const [userPowerPlants, setUserPowerPlants] = useState<Record<string, UserPowerPlant> | null>(null);
  const [userHashboards, setUserHashboards] = useState<number | null>(null);

  useEffect(() => {
    if (user_inventory) {
      setUserPowerPlants(user_inventory.user_power_plants as Record<string, UserPowerPlant>);

      const tokens = user_inventory.user_token_balances.find(([name, _number]) => name === "ASIC Gen 1") ?? ["ASIC Gen 1", 0];
      setUserHashboards(tokens[1])
    }
  });
  const owner = ((selected_plant && userPowerPlants) && userPowerPlants[selected_plant]) ?? null;
  const plantDetail = power_plants && selected_plant ? power_plants.find(pl => pl.id === selected_plant) : null;
  return (<Card>
    <CardHeader title={`Plant Detail`} />
    <CardContent>
      <Typography variant="h6">
        {selected_plant ? (owner ? 'Manage This Plant' : 'About This Plant') : 'Select a Plant on the Globe'}
      </Typography>
      {plantDetail && <Table>
        <TableHead>
          <TableRow>
            <TableCell>Detail</TableCell>
            <TableCell align="right"></TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          <TableRow>
            <TableCell>Plant Type</TableCell>
            <TableCell>
              <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[plantDetail.plant_type] }} /><p>{plantDetail.plant_type}</p>
            </TableCell>
          </TableRow>
          <TableRow>
            <TableCell>Location</TableCell>
            <TableCell >
              {`${plantDetail.coordinates[0] / COORDINATE_PRECISION}, ${plantDetail.coordinates[1] / COORDINATE_PRECISION}`}
            </TableCell>
          </TableRow>
          <TableRow>
            <TableCell >Hashrate</TableCell>
            <TableCell >{plantDetail.hashrate}</TableCell>
          </TableRow>
          <TableRow>
            <TableCell>Max Miners Supported</TableCell>
            <TableCell>{plantDetail.watts/100_000}</TableCell>
          </TableRow>
          <TableRow>
            <TableCell >Miners Allocated</TableCell>
            <TableCell >{plantDetail.miners}</TableCell>
          </TableRow>
        </TableBody>
      </Table>}
      <div>
        {selected_plant && plantDetail &&
          (owner && userHashboards && asic_token_id ? <div className="PlantOwnerOptions">
            <Typography variant="h6">Options</Typography>
            <MoveHashboards action={"ADD"} plant={plantDetail} user_hashboards={userHashboards} hashboard_pointer={asic_token_id} />
            <Divider />
            <SaleListingForm nft_id={plantDetail.id} currency={bitcoin_token_id} />
          </div> :
            <div className="NonOwnerOptions">
              <Typography variant="h6">Options</Typography>
              {plantDetail.for_sale && <PurchaseOfferForm nft_id={plantDetail.id} currency={bitcoin_token_id} listing_price={null} />}
            </div>)
        }
      </div>
    </CardContent>
  </Card>

  )
}