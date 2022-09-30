import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame, SwitchToGameProps } from './SwitchToGame';
import { KeySelector, KeySelectorProps } from './KeySelector';
import { MakeNewChain } from './MakeNewChain';
import "./AppHeader.css";
import React from 'react';
import { Typography } from '@mui/material';

export function AppHeader({ db_name_loaded, which_game_loaded, available_sequencers, available_keys, signing_key }: {
  db_name_loaded: [string, null | string] | null

} & SwitchToGameProps & KeySelectorProps) {
  return <div className="App-header">
    <Typography variant='h2'>1</Typography>
    <SwitchToDB {...{ db_name_loaded }}></SwitchToDB>
    {db_name_loaded &&
      <>
        <Typography variant='h2'>2</Typography>
        <MakeNewChain></MakeNewChain>
        <Typography variant='h2'>or</Typography>
        <SwitchToGame {...{ available_sequencers, which_game_loaded }} ></SwitchToGame>
        <Typography variant='h2'>3</Typography>
        <KeySelector {...{ available_keys, signing_key }}></KeySelector>
      </>
    }
  </div >;
}
