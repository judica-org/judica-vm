import { invoke } from '@tauri-apps/api';
import { PlantType } from './App';
import { GameMove } from './Types/GameMove';

export type SuccessfulTradeOutcome = {
  trading_pair: string,
  asset_player_purchased: string,
  amount_player_purchased: number,
  asset_player_sold: string,
  amount_player_sold: number,
}
export type UnsuccessfulTradeOutcome =
  { InsufficientTokens: string, InvalidTrade: undefined }
  | { InvalidTrade: string, InsufficientTokens: undefined }
  | "MarketSlipped";
export type TradeSimulation = { Ok: SuccessfulTradeOutcome, Err: undefined } | {
  Err: UnsuccessfulTradeOutcome
  , Ok: undefined
};
let game_synchronizer_invoked = false;
export const tauri_host = {
  get_move_schema: async () => {
    return await invoke("get_move_schema");
  },
  make_move_inner: async (nextMove: GameMove) => {
    return await invoke("make_move_inner", { nextMove });
  },
  game_synchronizer: async () => {
    if (game_synchronizer_invoked)
      return;
    game_synchronizer_invoked = true;
    invoke("game_synchronizer");
  },
  get_material_schema: async () => {
    return invoke("get_materials_schema");
  },
  switch_to_game: async (key: string) => {
    return invoke("switch_to_game", { key });
  },
  switch_to_db: async (appName: string, prefix: string | null) => {
    return invoke("switch_to_db", { appName, prefix });
  },
  set_signing_key: async (selected: string | null) => {
    return invoke("set_signing_key", { selected });
  },
  send_chat: async (chat: string) => {
    return invoke("send_chat", { chat })
  },
  make_new_chain: async (nickname: string) => {
    return invoke("make_new_chain", { nickname });
  },
  mint_power_plant_cost: async (scale: number, location: [number, number], plantType: PlantType) => {
    return invoke("mint_power_plant_cost", { scale, location, plantType });
  },
  super_mint: async (scale: number, location: [number, number], plantType: PlantType) => {
    return invoke("super_mint", { scale, location, plantType });
  },

  simulate_trade: async (pair: string, amounts: [number, number], trade: "buy" | "sell"): Promise<TradeSimulation> => {
    return invoke("simulate_trade", { pair, amounts, trade });
  }
};
