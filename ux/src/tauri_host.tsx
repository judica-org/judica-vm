import { invoke } from '@tauri-apps/api';

let game_synchronizer_invoked = false;
export const tauri_host = {
  get_move_schema: async () => {
    return await invoke("get_move_schema");
  },
  make_move_inner: async (nextMove: any, from: number) => {
    return await invoke("make_move_inner", { nextMove, from });
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
};
