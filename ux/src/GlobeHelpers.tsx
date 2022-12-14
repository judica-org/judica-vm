// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import * as React from 'react';
import FormGroup from '@mui/material/FormGroup';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import { lightGreen, orange, yellow } from '@mui/material/colors';
import { Typography } from '@mui/material';
import { EntityID } from './Types/GameMove';

export function PlantTypeSelect({ handleChange, plantTypes }: { handleChange: (event: React.ChangeEvent<HTMLInputElement>) => void, plantTypes: { 'Hydro': boolean, 'Solar': boolean, 'Flare': boolean } }) {
  const { Hydro, Solar, Flare } = plantTypes;

  return (
    <div >
      <Typography variant='h6'>Plant Type</Typography>
      <FormGroup sx={{ flexDirection: 'row' }}>
        <FormControlLabel control={<Checkbox checked={Hydro} onChange={handleChange} name="Hydro" />} label="Hydro" />
        <FormControlLabel control={<Checkbox checked={Solar} onChange={handleChange} name="Solar" sx={{
          color: yellow[800],
          '&.Mui-checked': {
            color: yellow[600],
          },
        }} />} label="Solar" />
        <FormControlLabel control={<Checkbox checked={Flare} onChange={handleChange} name="Flare" sx={{
          color: orange[800],
          '&.Mui-checked': {
            color: orange[600],
          },
        }} />} label="Flare" />
      </FormGroup>
    </div>
  );
}

export function PlantOwnerSelect({ handleChange, plantOwners, selectedPlantOwners, user_id }: { handleChange: (event: React.ChangeEvent<HTMLInputElement>) => void, plantOwners: Record<EntityID, true>, selectedPlantOwners: Record<EntityID, boolean>, user_id: EntityID | null }) {
  // for each owner, add a checkbox.
  return (
    <div>
      <Typography variant='h6'>Owners</Typography>
      <FormGroup sx={{
        flexDirection: 'row',
        flexWrap: 'wrap'
      }}>
        {
          Object.entries(plantOwners).map(([owner, _b]) => {
            return <FormControlLabel control={<Checkbox checked={selectedPlantOwners[owner]} onChange={handleChange} name={`${owner}`} sx={{
              color: lightGreen[800],
              '&.Mui-checked': {
                color: lightGreen[600],
              },
            }} />} label={owner === user_id ? `${owner} (ME)`: owner} key={owner} />
          })
        }
      </FormGroup>
    </div>
  )
}