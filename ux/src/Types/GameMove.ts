/**
 * Grab-Bag Enum of all moves
 *
 * N.B. we do the enum-of-struct-variant pattern to make serialization/schemas nicer.
 */
export type GameMove =
  | {
    heartbeat: Heartbeat
  }
  | TradeCoins
  | BuyNFTs
  | SellNFTs
  | SendCoins
  | RemoveTokens
  | SendALoggedChatMessageToAllPlayers
  | MintPowerPlantNFT
  | PurchaseMaterialsThenMintPowerPlantNFT
export type Heartbeat = []
export type TradingPairID = string
/**
* an EntityID is just a "pointer" we assign to all different types of things in our game, e.g. - Users - Token Contracts - NFTs - etc
*
* EntityIDs are global and unique within the game state
*/
export type EntityID = string
export type Chat = string
export type PlantType = "Solar" | "Hydro" | "Flare"

export interface TradeCoins {
  trade: Trade
}
export interface Trade {
  amount_a: number
  amount_b: number
  cap?: number | null
  pair: TradingPairID
  sell: boolean
  [k: string]: unknown
}
export interface BuyNFTs {
  purchase_n_f_t: PurchaseNFT
}
export interface PurchaseNFT {
  currency: EntityID
  limit_price: number
  nft_id: EntityID
  [k: string]: unknown
}
export interface SellNFTs {
  list_n_f_t_for_sale: ListNFTForSale
}
export interface ListNFTForSale {
  currency: EntityID
  nft_id: EntityID
  price: number
  [k: string]: unknown
}
export interface SendCoins {
  send_tokens: SendTokens
}
export interface SendTokens {
  amount: number
  currency: EntityID
  to: EntityID
  [k: string]: unknown
}
export interface RemoveTokens {
  remove_tokens: RemoveTokens1
}
export interface RemoveTokens1 {
  amount: number
  currency: EntityID
  nft_id: EntityID
  [k: string]: unknown
}
export interface SendALoggedChatMessageToAllPlayers {
  chat: Chat
}
export interface MintPowerPlantNFT {
  mint_power_plant: MintPowerPlant
}
export interface MintPowerPlant {
  location: [number, number]
  plant_type: PlantType
  /**
   * Size of the power plant
   */
  scale: number
  [k: string]: unknown
}
export interface PurchaseMaterialsThenMintPowerPlantNFT {
  super_mint_power_plant: MintPowerPlant
}
