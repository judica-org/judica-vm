import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect, useMemo, useRef } from 'react';
import './App.css';
import Form, { FormSubmit } from "@rjsf/core";
import EnergyExchange, { NFTSale } from './energy-exchange';
import WorkingGlobe from './WorkingGlobe';
import RawMaterialsMarket from './raw-materials';
import { tauri_host } from './tauri_host';
import { Chat } from './chat/Chat';
import { AppHeader } from './header/AppHeader';
import { Inventory } from './inventory/inventory';
import MintingModal from './mint-power-plant/MintingModal';
import { Box, Tab, Tabs } from '@mui/material';
import { TabPanelUnstyled, TabsUnstyled } from '@mui/base';


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

function GameBoard(props: { g: game_board }) {
  return <ul>
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
function App() {
  const [game_board, set_game_board] = useState<game_board | null>(null);

  const [current_tab, set_current_tab] = useState(1);
  useEffect(() => {
    const unlisten_game_board = appWindow.listen("game-board", (ev) => {
      console.log(['game-board-event'], ev);
      set_game_board(ev.payload as game_board)
    });
    tauri_host.game_synchronizer()
    return () => {
      (async () => {
        (await unlisten_game_board)();
      })();
    }
  }, [game_board]);

  return (
    <div>
      <AppHeader></AppHeader>
      <div className="App">
        <WorkingGlobe></WorkingGlobe>
        <Box>
          <Box sx={{ size: "sm" }}>
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
            <MintingModal />
          </Panel>
          <Panel index={2} current_index={current_tab}>
            {<EnergyExchange></EnergyExchange>}
          </Panel>
          <Panel index={3} current_index={current_tab}>
            <RawMaterialsMarket></RawMaterialsMarket>
          </Panel>
          <Panel index={4} current_index={current_tab}>
            <Inventory></Inventory>
          </Panel>
          <Panel index={5} current_index={current_tab}>
            <MoveForm></MoveForm>
          </Panel>
          <Panel index={6} current_index={current_tab}>
            <Chat></Chat>
          </Panel>
          <Panel index={7} current_index={current_tab}>
            {game_board && <GameBoard g={game_board}></GameBoard>}
          </Panel>
        </Box>
      </div >
    </div >
  );
}

export default App;

