import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame, SwitchToGameProps } from './SwitchToGame';
import { KeySelector, KeySelectorProps } from './KeySelector';
import "./AppHeader.css";
import React from 'react';
import { FormControl, FormControlLabel, Switch, Typography } from '@mui/material';
import { NewGame, NewGameProps } from './NewGame';
import { SwitchToHost, SwitchToHostProps } from './SwitchToHost';

export function AppHeader({ db_name_loaded, which_game_loaded,
  available_sequencers, available_keys,
  signing_key, join_code, join_password, game_host_service }: {
    db_name_loaded: [string, null | string] | null

  } & SwitchToGameProps & KeySelectorProps & NewGameProps & SwitchToHostProps) {
  const [new_or_old, set_new_or_old] = React.useState(false);
  const action = new_or_old ? "New" : "Existing";
  return <div className="App-header">
    <Typography variant='h2'>1</Typography>
    <SwitchToDB {...{ db_name_loaded }}></SwitchToDB>
    <SwitchToHost {...{ game_host_service }}></SwitchToHost>
    {db_name_loaded && game_host_service &&
      <>
        <FormControl>

          <Typography variant='h2'>2</Typography>
          <FormControlLabel control={
            <Switch value={new_or_old} onClick={(a) => { set_new_or_old(!new_or_old) }}></Switch>
          } label={`${action} Game`} />
        </FormControl>
        {new_or_old ?
          <NewGame {...{ join_code, join_password }}></NewGame> :
          <SwitchToGame {...{ available_sequencers, which_game_loaded }} ></SwitchToGame>}
      </>
    }
  </div >;
}
