import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame } from './SwitchToGame';
import { KeySelector } from './KeySelector';
import { MakeNewChain } from './MakeNewChain';
import "./AppHeader.css";
import React from 'react';
import { Typography } from '@mui/material';

export function AppHeader() {
  const [db_name_loaded, set_db_name_loaded] = React.useState<[string, string | null] | null>(null);
  return <div className="App-header">
    <Typography variant='h2'>1</Typography>
    <SwitchToDB {...{ db_name_loaded, set_db_name_loaded }}></SwitchToDB>
    {db_name_loaded &&
      <>
      <Typography variant='h2'>2</Typography>
        <MakeNewChain></MakeNewChain>
      <Typography variant='h2'>or</Typography>
        <SwitchToGame></SwitchToGame>
      <Typography variant='h2'>3</Typography>
        <KeySelector></KeySelector>
      </>
    }
  </div >;
}
