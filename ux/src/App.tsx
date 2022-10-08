import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect, useMemo, useRef } from 'react';
import './App.css';
import EnergyExchange, { NFTSale } from './energy-exchange/EnergyExchange';
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
import { EntityID, TradingPairID } from './Types/GameMove';
import { ManagePlant, PlantLabel } from './manage-plant/ManagePlant';
import MoveForm from './move-form/MoveForm';
import { EmittedAppState, GameBoard, UXPlantData } from './Types/Gameboard';
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


function ListGameBoard(props: { g: GameBoard | null }) {
  return props.g && <ul>
    <li>
      Init: {JSON.stringify(props.g.init)}
    </li>
    <li>
      New Users: {JSON.stringify(props.g.new_users_allowed)}
    </li>
    <li>
      User List: {JSON.stringify(props.g.users)}
    </li>
    <li>
      Root User: {JSON.stringify(props.g.root_user)}
    </li>
    <li>
      Bitcoin Token ID: {JSON.stringify(props.g.bitcoin_token_id)}
    </li>
    <li>
      Dollar Token ID: {JSON.stringify(props.g.dollar_token_id)}
    </li>
    <li>
      ERC20s: {JSON.stringify(props.g.erc20s)}
    </li>
    <li>
      Exchanges: {JSON.stringify(props.g.swap)}
    </li>
    <li>
      NFTs: {JSON.stringify(props.g.nfts)}
    </li>
    <li>
      NFT Sales: {JSON.stringify(props.g.nft_sales)}
    </li>
  </ul>;
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
function App() {
  const [location, setLocation] = useState<[number, number]>([0, 0]);
  const [selected_plant, set_selected_plant] = useState<EntityID | null>(null);
  const [current_tab, set_current_tab] = useState(1);
  const [root_state, set_root_state] = useState<null | EmittedAppState>(null);
  useEffect(() => {
    let cancel = setTimeout(() => { }, 0);
    const callback = async () => {
      set_root_state(await tauri_host.game_synchronizer());
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
  const game_board = root_state?.game_board ?? null;
  const user_inventory = root_state?.user_inventory ?? null;
  const listings = root_state?.energy_exchange ?? [];


  useEffect(() => {
    listen("globe-location", (ev: { payload: any }) => {
      console.log(["globe-location"], JSON.parse(ev.payload));
      setLocation(JSON.parse(ev.payload));
    });

    ListenPlantSelected((d) => {
      set_selected_plant(d)
      set_current_tab(8);
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
          game_host_service
        }}></DrawerAppBar>
        <div className="Content">
          <WorkingGlobe power_plants={power_plants}></WorkingGlobe>
          <Box className="DataDisplay">
            <Box sx={{ borderBottom: 1, borderColor: 'divider' }} className="DisplayContents">
              <Tabs onChange={(_ev, value) => set_current_tab(value)} scrollButtons="auto" variant="scrollable" value={current_tab}>
                <Tab value={1} label="Minting"></Tab>
                <Tab value={2} label="Energy Exchange"></Tab>
                <Tab value={3} label="Materials Market"></Tab>
                <Tab value={4} label="Inventory"></Tab>
                <Tab value={5} label="Raw Move"></Tab>
                <Tab value={6} label="Chat"></Tab>
                <Tab value={7} label="Board JSON"></Tab>
                <Tab value={8} label="Manage Plant"></Tab>
              </Tabs>
            </Box>
            <Panel index={1} current_index={current_tab} >
              <Minting power_plants={powerPlants} />
            </Panel>
            <Panel index={2} current_index={current_tab}>
              {<EnergyExchange listings={listings}></EnergyExchange>}
            </Panel>
            <Panel index={3} current_index={current_tab}>
              <RawMaterialsMarket materials={materials}></RawMaterialsMarket>
            </Panel>
            <Panel index={4} current_index={current_tab}>
              <Inventory userInventory={user_inventory}></Inventory>
            </Panel>
            <Panel index={5} current_index={current_tab}>
              <MoveForm></MoveForm>
            </Panel>
            <Panel index={6} current_index={current_tab}>
              <Chat chat_log={chat_log}></Chat>
            </Panel>
            <Panel index={7} current_index={current_tab}>
              <ListGameBoard g={game_board}></ListGameBoard>
            </Panel>
            <Panel index={8} current_index={current_tab}>
              <ManagePlant
                asic_token_id={game_board?.asic_token_id ?? null}
                selected_plant={selected_plant}
                power_plants={power_plants}
                user_inventory={user_inventory}
              />
            </Panel>
          </Box>
        </div>
      </div >
    </div >
  );
}

export default App;

