import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect, useMemo, useRef } from 'react';
import './App.css';
import Form, { FormSubmit } from "@rjsf/core";
import EnergyExchange, { NFTSale } from './energy-exchange/EnergyExchange';
import WorkingGlobe from './WorkingGlobe';
import RawMaterialsMarket from './raw-materials/RawMaterialsMarket';
import { tauri_host } from './tauri_host';
import { Chat } from './chat/Chat';
import { AppHeader } from './header/AppHeader';
import { Inventory } from './inventory/inventory';
import MintingModal from './mint-power-plant/MintingModal';
import { listen } from '@tauri-apps/api/event';
import { Box, Tab, Tabs } from '@mui/material';
import { TabPanelUnstyled, TabsUnstyled } from '@mui/base';
import React from 'react';
import DrawerAppBar from './menu-bar/MenuDrawer';
export type PlantType = 'Solar' | 'Hydro' | 'Flare';
export type PowerPlant = {
  id: number,
  plant_type: PlantType //how does PlantType enum show up
  watts: number,
  coordinates: number[],
  owner: number,
  has_miners: boolean,
  for_sale: boolean,
}

function MoveForm() {
  const [schema, set_schema] = useState<null | any>(null);

  useEffect(() => {
    (async () => {
      set_schema(await tauri_host.get_move_schema());
    })()

  }, []);
  console.log(schema);
  const handle_submit = (data: FormSubmit) => {
    // TODO: Submit from correct user
    const uid_n = uid.current?.valueAsNumber;
    if (uid_n)
      tauri_host.make_move_inner(data.formData, uid_n)
  };
  const schema_form = useMemo<JSX.Element>(() => {
    const customFormats = { "uint128": (s: string) => { return true; } };
    if (schema)
      return <Form schema={schema} noValidate={true} liveValidate={false} onSubmit={handle_submit} customFormats={customFormats}>
        <button type="submit">Submit</button>
      </Form>;
    else
      return <div></div>
  }
    , [schema]
  )
  const uid = useRef<null | HTMLInputElement>(null);
  return schema && <div className='MoveForm'>
    <div>
      <label>Player ID:</label>
      <input type={"number"} ref={uid}></input>
    </div>
    {schema_form}
  </div>;
}

type NFTs = {
  nfts: { nft_id: number, owner: number, transfer_count: number }[],
  power_plants: {
    id: number,
    plant_type: string //how does PlantType enum show up
    watts: number,
    coordinates: number[]
  }[]
}

type game_board = {
  erc20s: any,
  swap: any, // determine TS swap shape
  turn_count: number,
  alloc: any,
  users: Record<string, string>,
  nfts: NFTs,
  nft_sales: { nfts: NFTSale },
  player_move_sequences: Record<string, number>,
  init: boolean,
  new_users_allowed: boolean,
  bitcoin_token_id: null | string,
  dollar_token_id: null | string,
  root_user: null | string,
};

function GameBoard(props: { g: game_board | null }) {
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
  readonly trading_pair: {
    readonly asset_a: number;
    readonly asset_b: number;
  },
  readonly asset_a: string;
  readonly mkt_qty_a: number;
  readonly asset_b: string;
  readonly mkt_qty_b: number;
}

export type MaterialType = 'Steel' | 'Silicon' | 'Concrete';

export type MaterialPriceDisplay = {
  trading_pair: {
    asset_a: number;
    asset_b: number;
  },
  material_type: MaterialType;
  price: number | 'not available';
  currency: string;
}

export type UserPowerPlant = PowerPlant & {
  readonly hashrate: number | null;
}

export type UserInventory = {
  user_power_plants: Record<string, UserPowerPlant>,
  user_token_balances: [string, number][]
}
function App() {
  const [game_board, set_game_board] = useState<game_board | null>(null);
  const [location, setLocation] = useState<[number, number]>([0, 0]);

  const [materials, set_materials] = useState<MaterialPriceDisplay[]>([]);
  const [current_tab, set_current_tab] = useState(1);
  const [userInventory, setUserInventory] = useState<UserInventory | null>(null);
  const [chat_log, set_chat_log] = React.useState<[number, number, string][]>([]);
  const [listings, setListings] = useState<NFTSale[]>([]);
  useEffect(() => {
    const unlisten_game_board = appWindow.listen("game-board", (ev) => {
      console.log(['game-board-event'], ev);
      set_game_board(ev.payload as game_board)
    });


    const unlisten_material_prices = appWindow.listen("materials-price-data", (ev) => {
      console.log(ev);
      const materials_data = ev.payload as MaterialPriceData[];
      const transformed: MaterialPriceDisplay[] = materials_data.map(({ trading_pair, asset_a, mkt_qty_a, asset_b, mkt_qty_b }) => {
        return {
          trading_pair,
          material_type: asset_a as MaterialType,
          price: Math.round(mkt_qty_b / mkt_qty_a),
          currency: asset_b,
        }
      })
      set_materials(transformed)
    });

    const unlisten_user_inventory = appWindow.listen("user-inventory", (ev) => {
      console.log(['user-inventory'], ev);
      setUserInventory(ev.payload as UserInventory);
    });

    const unlisten_chat_log = appWindow.listen("chat-log", (ev) => {
      console.log("Chat:", ev.payload);
      const new_msgs = ev.payload as typeof chat_log;
      set_chat_log(new_msgs)
    })

    const unlisten_energy_exchange = appWindow.listen("energy-exchange", (ev) => {
      console.log(['energy-exchange'], ev);
      setListings(ev.payload as NFTSale[]);
    });
    tauri_host.game_synchronizer()
    return () => {
      (async () => {
        const unlisten_all = await Promise.all([unlisten_chat_log, unlisten_energy_exchange, unlisten_game_board, unlisten_material_prices, unlisten_user_inventory, unlisten_energy_exchange]);
        for (const u of unlisten_all) {
          u()
        }
      })();
    }
  }, []);



  useEffect(() => {
    listen("globe-location", (ev: { payload: any }) => {
      console.log(["globe-location"], JSON.parse(ev.payload));
      setLocation(JSON.parse(ev.payload));
    });
  });

  return (
    <div>
      <div className="App">
        <DrawerAppBar></DrawerAppBar>
        <div className="Content">
        <WorkingGlobe></WorkingGlobe>
        <Box className="DataDisplay">
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <Tabs onChange={(_ev, value) => set_current_tab(value)} scrollButtons="auto" variant="scrollable" value={current_tab}>
              <Tab value={1} label="Minting"></Tab>
              <Tab value={2} label="Energy Exchange"></Tab>
              <Tab value={3} label="Materials Market"></Tab>
              <Tab value={4} label="Inventory"></Tab>
              <Tab value={5} label="Raw Move"></Tab>
              <Tab value={6} label="Chat"></Tab>
              <Tab value={7} label="Board JSON"></Tab>
            </Tabs>
          </Box>
          <Panel index={1} current_index={current_tab}>
            <MintingModal location={location} />
          </Panel>
          <Panel index={2} current_index={current_tab}>
            {<EnergyExchange listings={listings}></EnergyExchange>}
          </Panel>
          <Panel index={3} current_index={current_tab}>
            <RawMaterialsMarket materials={materials}></RawMaterialsMarket>
          </Panel>
          <Panel index={4} current_index={current_tab}>
            <Inventory userInventory={userInventory}></Inventory>
          </Panel>
          <Panel index={5} current_index={current_tab}>
            <MoveForm></MoveForm>
          </Panel>
          <Panel index={6} current_index={current_tab}>
            <Chat chat_log={chat_log}></Chat>
          </Panel>
          <Panel index={7} current_index={current_tab}>
            <GameBoard g={game_board}></GameBoard>
          </Panel>
        </Box>
        </div>
      </div >
    </div >
  );
}

export default App;

