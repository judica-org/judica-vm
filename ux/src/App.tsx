import { invoke } from '@tauri-apps/api';
import { appWindow } from '@tauri-apps/api/window';
import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import './App.css';
import Form, { FormSubmit } from "@rjsf/core";
import { PowerPlant, PowerPlants } from './power-plant-list';
import EnergyExchange, { NFTSale } from './energy-exchange';
import Globe from 'react-globe.gl';
// import { MakeGlobe } from './globe-display';
import CustomGlobe, { MakeGlobe } from './CustomGlobe';

function MoveForm() {
  const [schema, set_schema] = useState<null | any>(null);

  useEffect(() => {
    (async () => {
      set_schema(await invoke("get_move_schema"));
    })()

  }, []);
  console.log(schema);
  const handle_submit = (data: FormSubmit) => {
    // TODO: Submit from correct user
    invoke("make_move_inner", { nextMove: data.formData, from: uid.current?.valueAsNumber })

  };
  const customFormats = { "uint128": (s: string) => { return true; } };
  const schema_form = useMemo<JSX.Element>(() => {
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

let invoked = false;
const invoke_once = () => {
  if (invoked) return;
  invoked = true;
  invoke("game_synchronizer")
}

function App() {
  const [game_board, set_game_board] = useState<game_board | null>(null);
  const [power_plants, set_power_plants] = useState<PowerPlant[]>([]); // use empty list for now so it will render
  const [countries, setCountries] = useState<{features: any[]}>({ features: [] });

  useEffect(() => {
    setCountries(countries);
    console.log("loaded countries", countries.features[0]);
    const unlisten_game_board = appWindow.listen("game-board", (ev) => {
      console.log(['game-board-event'], ev);
      set_game_board(JSON.parse(ev.payload as string) as game_board)
    });

    invoke_once()
    return () => {
      (async () => {
        (await unlisten_game_board)();
      })();
    }
  }, [game_board, countries, setCountries]);

  // update deps, 
  // image file should be local
  // tauri toolbar - add a reload button?
  // when there's no background its unhappy
  // bug in the FE rust logic - because no game initialized just keeps checking for game. <-fix ux-scheduler branch - merge in.


  return (
    <div className="App">
      {game_board && <GameBoard g={game_board}></GameBoard>}
      {power_plants && <PowerPlants power_plants={power_plants}></PowerPlants>}
      {<Globe width={500}
        height={500}
        globeImageUrl={"//unpkg.com/three-globe/example/img/earth-dark.jpg"}
        hexPolygonsData={countries.features}
        hexPolygonResolution={3}
        hexPolygonMargin={0.3}
        hexPolygonColor={useCallback(() => "#1b66b1", [])}
      ></Globe>}
      {<CustomGlobe></CustomGlobe>}
      {<MakeGlobe></MakeGlobe>}
      {<EnergyExchange listings={[{
        price: 937,
        currency: 'donuts',
        seller: 95720486,
        transfer_count: 2
      }, {
        price: 424,
        currency: 'cookies',
        seller: 3058572037,
        transfer_count: 1
      }]}></EnergyExchange>}
      <MoveForm></MoveForm>
    </div>
  );
}

export default App;
