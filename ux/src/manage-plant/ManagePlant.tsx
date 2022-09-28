import { Card, CardHeader, CardContent, Typography, Table, TableBody, TableCell, TableHead, TableRow } from "@mui/material";
import FactoryIcon from '@mui/icons-material/Factory';
import { Event, listen } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react"
import { UserPowerPlant, UserInventory, game_board } from "../App";
import FormModal from "../form-modal/FormModal";
import { plant_type_color_map } from "../util";
import { MoveHashboards } from "../move-hashboards/MoveHashboards";

type PlantLabel = { readonly id: number, readonly owner: string, readonly watts: string, readonly for_sale: boolean };

export const ManagePlant = () => {
  /* Need:
  1. Plant from map
  2. Plants loaded with game-board
   */
  const [plantFromMap, setPlantFromMap] = useState<PlantLabel | null>(null);
  const [powerPlants, setPowerPlants] = useState<UserPowerPlant[] | null>(null);
  const [userPowerPlants, setUserPowerPlants] = useState<Record<string, UserPowerPlant> | null>(null);
  const [userHashboards, setUserHashboards] = useState<number | null>(null);
  const [hashboardPointer, setHashboardPointer] = useState<string | null>(null);
  listen("plant-selected", (ev) => {
    setPlantFromMap(ev.payload as PlantLabel)
  });

  useEffect(() => {
    const unlisten_power_plants = appWindow.listen("power-plants", (ev) => {
      console.log(['power-plants'], ev);
      setPowerPlants(ev.payload as UserPowerPlant[])
    });

    const unlisten_user_inventory = appWindow.listen("user-inventory", (ev: Event<UserInventory>) => {
      console.log(['user-inventory'], ev);
      setUserPowerPlants(ev.payload.user_power_plants as Record<string, UserPowerPlant>);

      const tokens = ev.payload.user_token_balances.find(([name, _number]) => name === "ASIC Gen 1") ?? ["ASIC Gen 1", 0];
      setUserHashboards(tokens[1])
    });

    const unlisten_price_data = appWindow.listen("game-board", (ev: Event<game_board>) => {
      console.log(['game-board'], ev);

      setHashboardPointer(ev.payload.asic_token_id)
    });


    return () => {
      (async () => {
        (await unlisten_power_plants)();
        (await unlisten_user_inventory)();
        (await unlisten_price_data)();
      })();
    }
  })
  const owner = ((plantFromMap && userPowerPlants) && userPowerPlants[plantFromMap.id]) ?? null;
  const plantDetail = plantFromMap && powerPlants ? powerPlants.find(pl => pl.id === plantFromMap.id) : null;
  return (<Card>
    <CardHeader title={`Plant Detail`} />
    <CardContent>
      <Typography variant="h6">
        {plantFromMap ? (owner ? 'Manage This Plant' : 'About This Plant') : 'Select a Plant on the Globe'}
      </Typography>
      {plantDetail && <Table>
        <TableHead>
          <TableCell>Detail</TableCell>
          <TableCell align="right"></TableCell>
        </TableHead>
        <TableBody>
          <TableRow>
            <TableCell>Plant Type</TableCell>
            <TableCell>
              <FactoryIcon className='sale-factory-icon' sx={{ color: plant_type_color_map[plantDetail.plant_type] }} /><p>{plantDetail.plant_type}</p>
            </TableCell>
          </TableRow>
          <TableCell>Location</TableCell>
          <TableCell >
            {plantDetail.coordinates}
          </TableCell>
          <TableRow>
            <TableCell >Hashrate</TableCell>
            <TableCell >{plantDetail.hashrate}</TableCell>
          </TableRow>
          <TableCell >Miners Allocated</TableCell>
          <TableCell align="right">{plantDetail.miners}</TableCell>

          <TableRow>
            <TableCell >More Actions</TableCell>
            <TableCell align="right"></TableCell>

          </TableRow>
        </TableBody>
      </Table>}
      <div>
        {plantFromMap && (
          owner && plantDetail && hashboardPointer && userHashboards ? <div className="PlantOwnerOptions">
            <Typography variant="h6">Options</Typography>
            <MoveHashboards action={"ADD"} plant={plantDetail} user_hashboards={userHashboards} hashboard_pointer={hashboardPointer} />
            <FormModal action={"Sell Plant"} title={'Sell Plant'} nft_id={plantDetail.id} />
          </div> :
            <div className="NonOwnerOptions">
              <Typography variant="h6">Options</Typography>
              <FormModal action="Purchase Plant" title={"Purchase Plant"} nft_id={plantFromMap.id} />
            </div>)
        }
      </div>
    </CardContent>
  </Card>

  )
}