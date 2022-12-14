// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { invoke } from '@tauri-apps/api';
import { PlantType } from './App';
import { EmittedAppState, UXUserInventory } from './Types/Gameboard';
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
    console.log(["make-move-inner"], nextMove);
    return await invoke("make_move_inner", { nextMove });
  },
  game_synchronizer: async (): Promise<EmittedAppState> => {
    return await invoke("game_synchronizer");
  },
  get_material_schema: async () => {
    return invoke("get_materials_schema");
  },
  get_inventory_by_key: async (userKey: string): Promise<UXUserInventory> => {
    return invoke("get_inventory_by_key", { userKey });
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
  join_existing_game: async (nickname: string, code: string) => {
    return invoke("make_new_chain", { nickname, code });
  },
  make_new_game: async (nickname: string, minutes: number) => {
    return invoke("make_new_game", { nickname, minutes });
  },
  mint_power_plant_cost: async (scale: number, location: [number, number], plantType: PlantType) => {
    console.log("building plant", {scale, location, plantType});
    return invoke("mint_power_plant_cost", { scale, location, plantType });
  },
  super_mint: async (scale: number, location: [number, number], plantType: PlantType) => {
    return invoke("super_mint", { scale, location, plantType });
  },

  simulate_trade: async (pair: string, amounts: [number, number], trade: "buy" | "sell"): Promise<TradeSimulation> => {
    return invoke("simulate_trade", { pair, amounts, trade });
  },
  set_game_host: async (g: { url: string, port: number }): Promise<void> => {
    return invoke("set_game_host", { g });
  },
  finalize_game: async (args: FinishArgs): Promise<void> => {
    return invoke("finalize_game", { args });
  },
  disconnect_game: async (): Promise<void> => {
    return await invoke("disconnect_game", {});
  },
  disconnect_game_host: async (): Promise<void> => {
    return await invoke("disconnect_game_host", {});
  }
};

export type FinishArgs = {
  passcode: string,
  code: string,
  start_amount: number,
}
