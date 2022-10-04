import { Paper, Typography } from '@mui/material';
import { Event, listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { UserPowerPlant } from '../App';
import MintingForm from './MintingForm';

function Minting({ power_plants }: { power_plants: UserPowerPlant[] | null }) {
  const [location, setLocation] = useState<[number, number] | null>(null);
  const [proximity_ok, set_proximity_ok] = useState<boolean>(true);

  useEffect(() => {
    listen("globe-location", (ev: Event<string>) => {
      console.log(["globe-location-received"], ev.payload)
      setLocation(JSON.parse(ev.payload))
    });

    if (power_plants && location) {
      const p = power_plants.filter((plant) => Math.abs(plant.coordinates[0] - location[0]) > 2 || Math.abs(plant.coordinates[1] - location[1]) > 2).length ? false : true;
      set_proximity_ok(p);
    }
  }, [location])


  return (
    <div className="MintingModal">
      <Paper>
        {!proximity_ok && <Typography variant='body1'>Selected location is too close to an existing plant</Typography>}
        {!location && <Typography variant='body1'>Select a location on the Globe to Mint a Power Plant</Typography>}
        {location && proximity_ok ?
          <MintingForm location={location} /> : null}
      </Paper>
    </div>
  )
}

export default Minting;