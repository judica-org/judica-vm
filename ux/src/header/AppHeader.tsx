import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame, SwitchToGameProps } from './SwitchToGame';
import { KeySelector, KeySelectorProps } from './KeySelector';
import "./AppHeader.css";
import React from 'react';
import { FormControl, FormGroup, FormLabel, IconButton, ToggleButton, ToggleButtonGroup } from '@mui/material';
import { NewGame, NewGameProps } from './NewGame';
import { SwitchToHost, SwitchToHostProps } from './SwitchToHost';
import { ContentCopy, FiberNew, List, RemoveCircleOutline } from '@mui/icons-material';
import { tauri_host } from '../tauri_host';

export function AppHeader({ db_name_loaded, which_game_loaded,
  available_sequencers, available_keys,
  signing_key, join_code, join_password, game_host_service }: {
    db_name_loaded: [string, null | string] | null

  } & SwitchToGameProps & KeySelectorProps & NewGameProps & SwitchToHostProps) {
  return <div className="App-header">
    <SwitchToDB {...{ db_name_loaded }}></SwitchToDB>
    <SwitchToHost {...{ game_host_service }}></SwitchToHost>
    <GamePicker {...{
      db_name_loaded: db_name_loaded !== null,
      available_sequencers, which_game_loaded, join_code,
      join_password, game_host_service: game_host_service !== null
    }}></GamePicker>
    <KeySelector {...{ available_sequencers, which_game_loaded, signing_key, available_keys }} disabled={which_game_loaded === null}></KeySelector>
  </div >;

}
function GamePicker({ db_name_loaded, available_sequencers, which_game_loaded, join_code, join_password, game_host_service }: { db_name_loaded: boolean, game_host_service: boolean } & NewGameProps & SwitchToGameProps) {
  const [new_or_old, set_new_or_old] = React.useState(false);
  const action = new_or_old ? "New" : "Existing";
  return <FormControl disabled={!(db_name_loaded)}>
    {which_game_loaded !== null &&
      <FormGroup row>
        <FormLabel sx={{ wordBreak: "break-word" }}>Selected Sequencer: {which_game_loaded}</FormLabel>
        <IconButton  onClick={() => tauri_host.disconnect_game()}> <RemoveCircleOutline></RemoveCircleOutline> </IconButton>
        <IconButton onClick={() => window.navigator.clipboard.writeText(which_game_loaded)}><ContentCopy></ContentCopy></IconButton>
      </FormGroup>
    }
    {null === which_game_loaded &&
      <>
        <FormGroup>
          <FormLabel>
            <span style={{ fontWeight: new_or_old ? "bold" : "normal" }}>New </span>
            or
            <span style={{ fontWeight: !new_or_old ? "bold" : "normal" }}> Existing </span>
            Game
          </FormLabel>
          <ToggleButtonGroup value={new_or_old}
            exclusive
            onChange={(a, newValue) => { newValue !== null && set_new_or_old(newValue); }}
          >
            <ToggleButton
              value={true}><FiberNew></FiberNew> </ToggleButton>
            <ToggleButton value={false}><List></List></ToggleButton>
          </ToggleButtonGroup>
        </FormGroup>
        {new_or_old ?
          <NewGame ext_disabled={!game_host_service} {...{ join_code, join_password }}></NewGame> :
          <SwitchToGame {...{ available_sequencers, which_game_loaded }}></SwitchToGame>}
      </>
    }
  </FormControl>;
}
