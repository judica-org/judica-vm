import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame, SwitchToGameProps } from './SwitchToGame';
import { KeySelector, KeySelectorProps } from './KeySelector';
import "./AppHeader.css";
import React from 'react';
import { FormControl, FormControlLabel, FormGroup, FormLabel, Switch, ToggleButton, ToggleButtonGroup, Typography } from '@mui/material';
import { NewGame, NewGameProps } from './NewGame';
import { SwitchToHost, SwitchToHostProps } from './SwitchToHost';
import { FiberNew, List } from '@mui/icons-material';

export function AppHeader({ db_name_loaded, which_game_loaded,
  available_sequencers, available_keys,
  signing_key, join_code, join_password, game_host_service }: {
    db_name_loaded: [string, null | string] | null

  } & SwitchToGameProps & KeySelectorProps & NewGameProps & SwitchToHostProps) {
  const [new_or_old, set_new_or_old] = React.useState(false);
  const action = new_or_old ? "New" : "Existing";
  return <div className="App-header">
    <SwitchToDB {...{ db_name_loaded }}></SwitchToDB>
    <SwitchToHost {...{ game_host_service }}></SwitchToHost>
    <FormControl disabled={!(db_name_loaded )} >
      <FormGroup>
        <FormLabel>
          <span style={{ fontWeight: new_or_old ? "bold" : "normal" }}>New </span>
          or
          <span style={{ fontWeight: !new_or_old ? "bold" : "normal" }}> Existing </span>
          Game
        </FormLabel>
        <ToggleButtonGroup value={new_or_old}
          exclusive
          onChange={(a, newValue) => { newValue !== null && set_new_or_old(newValue) }}
        >
          <ToggleButton
            value={true}><FiberNew></FiberNew> </ToggleButton>
          <ToggleButton value={false}><List></List></ToggleButton>
        </ToggleButtonGroup>
      </FormGroup>
      {new_or_old ?
        <NewGame ext_disabled={!game_host_service} {...{ join_code, join_password }}></NewGame> :
        <SwitchToGame {...{ available_sequencers, which_game_loaded }} ></SwitchToGame>}
    </FormControl>
    <KeySelector {...{ available_sequencers, which_game_loaded, signing_key, available_keys }} disabled={which_game_loaded === null}></KeySelector>
  </div >;
}
