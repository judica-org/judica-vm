// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useState, useEffect } from 'react';
import './App.css';
import EnergyExchange from './energy-exchange/EnergyExchange';
import WorkingGlobe from './WorkingGlobe';
import RawMaterialsMarket from './raw-materials/RawMaterialsMarket';
import { tauri_host } from './tauri_host';
import { Chat } from './chat/Chat';
import { Inventory } from './inventory/inventory';
import Minting from './mint-power-plant/Minting';
import { listen } from '@tauri-apps/api/event';
import { Box, Tab, Tabs } from '@mui/material';
import React from 'react';
import DrawerAppBar from './menu-bar/MenuDrawer';
import { EntityID } from './Types/GameMove';
import { ManagePlant } from './manage-plant/ManagePlant';
import MoveForm from './move-form/MoveForm';
import { EmittedAppState, LogEvent, UXPlantData } from './Types/Gameboard';
import { EventLog } from './event-log/EventLog';
import FooterTicker from './footer-ticker/FooterTicker';
import { AppHeader } from './header/AppHeader';
import { Backpack, BugReport, ChatBubble, ElectricBoltSharp, Settings, Shop2Sharp, StorefrontSharp } from '@mui/icons-material';
import { ListGameBoard } from './ListGameBoard';
export type PlantType = 'Solar' | 'Hydro' | 'Flare';

export const PLANT_SELECTED_EVENT = "PlantSelected";
export const PlantSelected = (d: EntityID) => {
  let ev = new CustomEvent(PLANT_SELECTED_EVENT, { detail: d, bubbles: false });
  document.dispatchEvent(ev);
}
declare global {
  interface DocumentEventMap {
    PlantSelected: CustomEvent<EntityID>,
  }
}
export const ListenPlantSelected = (f: (d: EntityID) => void) => {
  document.addEventListener(PLANT_SELECTED_EVENT, (ev: CustomEvent<EntityID>) => {
    f(ev.detail)
  })
}


function Panel(props: React.PropsWithChildren & { index: number, current_index: number }) {
  return <div hidden={props.index !== props.current_index}>
    {props.index === props.current_index && props.children}
  </div>
}

export type MaterialPriceData = {
  readonly trading_pair: string,
  readonly asset_a: string;
  readonly mkt_qty_a: number;
  readonly asset_b: string;
  readonly mkt_qty_b: number;
  readonly display_asset: string;
}

export type MaterialPriceDisplay = {
  trading_pair: string,
  asset_a: string;
  asset_b: string;
  display_asset: string,
  price_a_b_b_a: [number, number],
}

export type UserPowerPlant = UXPlantData;

export type UserInventory = {
  user_power_plants: Record<string, UserPowerPlant>,
  user_token_balances: [string, number][]
}
type TradingPairIDParsed = {
  asset_a: number,
  asset_b: number,
}
export function parse_trading_pair(s: string): TradingPairIDParsed {
  const [asset_a, asset_b] = s.split(":");
  return { asset_a: parseInt(asset_a, 16), asset_b: parseInt(asset_b, 16) }
}
export function trading_pair_to_string(s: TradingPairIDParsed): string {
  return `${s.asset_a.toString(16)}:${s.asset_b.toString(16)}`
}

export function flip_trading_pair(s: TradingPairIDParsed): TradingPairIDParsed {
  return { asset_a: s.asset_b, asset_b: s.asset_a }
}

const getLastMovesByPlayer = (log: [number, EntityID, LogEvent][]): string[] => {
  const recent_moves = log.reduceRight((acc: { [key: string]: string }, [_seq, player, event]) => {
    if (!acc[player]) {
      acc = { ...acc, [player]: JSON.stringify(event) }
    }
    return acc;
  }, {});
  return Object.entries(recent_moves).map(([player, event]) => `${player} last move: ${event}`);
}

const findFinishLog = (game_event_log: [number, string, LogEvent][]): boolean => {
  const finish_event = game_event_log.find(([_num, _player, event]) => JSON.stringify(event).includes("GameIsFinished"));
  return !!finish_event;
}

function App() {
  const [location, setLocation] = useState<[number, number]>([0, 0]);
  const [selected_plant, set_selected_plant] = useState<EntityID | null>(null);
  const [current_tab, set_current_tab] = useState(1);
  const [current_tab_plants, set_current_tab_plants] = useState(1);
  const [current_tab_nested, set_current_tab_nested] = useState(1);
  const [root_state, set_root_state] = useState<null | EmittedAppState>(null);
  // reset the tab selection on the nested tab on nav away
  useEffect(() => set_current_tab_nested(1), [current_tab]);
  useEffect(() => {
    let cancel = setTimeout(() => { }, 0);
    const callback = async () => {
      const newLocal = await tauri_host.game_synchronizer();
      set_root_state(newLocal);
      console.log(["root-state"], newLocal);
      cancel = setTimeout(callback, 5000);
    };
    callback();
    return () => {
      clearTimeout(cancel)
    }

  }, []);

  const materials: MaterialPriceDisplay[] = root_state?.materials_price_data?.map(({ trading_pair, asset_a, mkt_qty_a, asset_b, mkt_qty_b, display_asset }) => {
    return {
      trading_pair,
      price_a_b_b_a: [mkt_qty_b / mkt_qty_a, mkt_qty_a / mkt_qty_b],
      asset_a: asset_a,
      asset_b: asset_b,
      display_asset
    }
  }) ?? [];

  const db_name_loaded = root_state?.db_connection ?? null;
  const available_sequencers = root_state?.available_sequencers ?? [];

  const which_game_loaded = root_state?.host_key ?? null;
  const signing_key = root_state?.signing_key ?? null;

  const available_keys = root_state?.user_keys ?? [];
  const join_code = root_state?.pending?.join_code ?? null;
  const join_password = root_state?.pending?.password ?? null;
  const game_host_service = root_state?.game_host_service ?? null;
  const power_plants = root_state?.power_plants ?? [];
  const chat_log = root_state?.chat_log ?? [];
  const game_event_log = root_state?.game_board?.event_log ?? [];
  const game_board = root_state?.game_board ?? null;
  const user_inventory = root_state?.user_inventory ?? null;
  const listings = root_state?.energy_exchange ?? [];
  const user_id = (signing_key && root_state?.game_board?.users_by_key) ? root_state?.game_board?.users_by_key[signing_key] : null;
  const elapsed_time = root_state?.game_board?.elapsed_time ?? null;
  const finish_time = root_state?.game_board?.finish_time ?? null;
  const player_status = game_event_log.length ? getLastMovesByPlayer(game_event_log) : ["No moves to show"];
  const is_finished = game_event_log.length ? findFinishLog(game_event_log) : false;
  const player_key_map = root_state?.game_board?.users_by_key ?? {};
  console.log(["game-event-log"], root_state?.game_board?.event_log || "event log is empty");

  useEffect(() => {
    listen("globe-location", (ev: { payload: any }) => {
      console.log(["globe-location"], JSON.parse(ev.payload));
      setLocation(JSON.parse(ev.payload));
      set_current_tab(1);
      set_current_tab_nested(1);
    });

    ListenPlantSelected((d) => {
      set_selected_plant(d)
      set_current_tab(1);
      set_current_tab_plants(3);
    })
  });

  return (
    <div>
      <div className="App">
        <DrawerAppBar {...{
          db_name_loaded,
          available_sequencers, which_game_loaded,
          signing_key, available_keys,
          join_code, join_password,
          game_host_service,
          user_id,
          elapsed_time,
          finish_time,
          is_finished
        }}></DrawerAppBar>
        <div className="Content">
          <WorkingGlobe power_plants={power_plants} user_id={user_id}></WorkingGlobe>
          <Box className="DataDisplay">
            <Box sx={{ borderBottom: 1, borderColor: 'divider' }} className="DisplayContents">
              <Tabs onChange={(_ev, value) => set_current_tab(value)} scrollButtons="auto" variant="scrollable" value={current_tab}>
                <Tab value={10} icon={<Settings></Settings>}></Tab>
                <Tab value={1} icon={<ElectricBoltSharp></ElectricBoltSharp>}></Tab>
                <Tab value={3} icon={<StorefrontSharp></StorefrontSharp>}></Tab>
                <Tab value={4} icon={<Backpack></Backpack>}></Tab>
                <Tab value={6} icon={<ChatBubble></ChatBubble>}></Tab>
                <Tab value={5} icon={<BugReport></BugReport>}></Tab>
              </Tabs>
            </Box>
            <Panel index={1} current_index={current_tab} >
              <Tabs onChange={(_ev, value) => set_current_tab_plants(value)} scrollButtons="auto" variant="fullWidth" value={current_tab_plants} textColor="secondary" indicatorColor="secondary">
                <Tab value={1} label="Build Plants"></Tab>
                <Tab value={2} label="Buy/Sell Plants"></Tab>
                <Tab value={3} label="Manage Plant"></Tab>
              </Tabs>
              <Panel index={1} current_index={current_tab_plants} >
                <Minting />
              </Panel>
              <Panel index={2} current_index={current_tab_plants}>
                {<EnergyExchange listings={listings}></EnergyExchange>}
              </Panel>
              <Panel index={3} current_index={current_tab_plants}>
                <ManagePlant
                  asic_token_id={game_board?.asic_token_id ?? null}
                  bitcoin_token_id={game_board?.bitcoin_token_id ?? null}
                  selected_plant={selected_plant}
                  power_plants={power_plants}
                  user_inventory={user_inventory}
                />
              </Panel>
            </Panel>
            <Panel index={3} current_index={current_tab}>
              <RawMaterialsMarket materials={materials}></RawMaterialsMarket>
            </Panel>
            <Panel index={4} current_index={current_tab}>
              <Inventory player_key_map={player_key_map} signing_key={signing_key} currency={game_board?.bitcoin_token_id ?? null} hashboard_pointer={game_board?.asic_token_id ?? null}></Inventory>
            </Panel>
            <Panel index={5} current_index={current_tab}>
              <Tabs onChange={(_ev, value) => set_current_tab_nested(value)} scrollButtons="auto" variant="fullWidth" value={current_tab_nested} textColor="secondary" indicatorColor="secondary">
                <Tab value={1} label="Raw Move"></Tab>
                <Tab value={2} label="Event Log"></Tab>
                <Tab value={3} label="Board JSON"></Tab>
              </Tabs>
              <Panel index={1} current_index={current_tab_nested}>
                <MoveForm></MoveForm>
              </Panel>
              <Panel index={2} current_index={current_tab_nested}>
                <EventLog game_event_log={game_event_log}></EventLog>
              </Panel>
              <Panel index={3} current_index={current_tab_nested}>
                <ListGameBoard g={game_board}></ListGameBoard>
              </Panel>
            </Panel>
            <Panel index={6} current_index={current_tab}>
              <Chat chat_log={chat_log}></Chat>
            </Panel>
            <Panel index={10} current_index={current_tab}>
              <AppHeader {...{
                available_sequencers, which_game_loaded,
                db_name_loaded, signing_key,
                available_keys, join_code, join_password,
                game_host_service
              }}></AppHeader>
            </Panel>
          </Box>
        </div>
        <FooterTicker player_status={player_status} />
      </div >
    </div >
  );
}

export default App;
