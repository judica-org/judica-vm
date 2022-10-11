import { Paper, Typography } from '@mui/material';
import { Event, listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import MintingForm from './MintingForm';

function Minting() {
  const [location, setLocation] = useState<[number, number] | null>(null);

  useEffect(() => {
    listen("globe-location", (ev: Event<string>) => {
      console.log(["globe-location-received"], ev.payload)
      setLocation(JSON.parse(ev.payload))
    })
  })

  return (
    <div className="MintingModal">
      <Paper>
        {location ?
          <MintingForm location={location} /> :
          <Typography variant='body1'>Select a location on the Globe to Build a Power Plant</Typography>}
      </Paper>
    </div>
  )
}

export default Minting;