/**
 * Grab-Bag Enum of all movesnnN.B. we do the enum-of-struct-variant pattern to make serialization/schemas nicer.
 */
 export type GameMove =
 | {
     heartbeat: Heartbeat
   }
 | TradeCoins
 | BuyNFTs
 | SellNFTs
 | SendCoins
 | SendALoggedChatMessageToAllPlayers
 | MintPowerPlantNFT
 | PurchaseMaterialsThenMintPowerPlantNFT
export type Heartbeat = []
/**
* A special Pointer designed for safer access to the NFTRegistry (prevent confusion with EntityID type)nnTODO: Guarantee validity for a given NFTRegistry
*/
export type NftPtr = number
export type Chat = string
export type PlantType = "Solar" | "Hydro" | "Flare"

export interface TradeCoins {
 trade: Trade
}
export interface Trade {
 amount_a: number
 amount_b: number
 pair: TradingPairID
 [k: string]: unknown
}
/**
* A TradingPair, not guaranteed to be normalized (which can lead to weird bugs) Auto-canonicalizing is undesirable since a user might specify elsewhere in corresponding order what their trade is.
*/
export interface TradingPairID {
 asset_a: number
 asset_b: number
 [k: string]: unknown
}
export interface BuyNFTs {
 purchase_n_f_t: PurchaseNFT
}
export interface PurchaseNFT {
 currency: number
 limit_price: number
 nft_id: NftPtr
 [k: string]: unknown
}
export interface SellNFTs {
 list_n_f_t_for_sale: ListNFTForSale
}
export interface ListNFTForSale {
 currency: number
 nft_id: NftPtr
 price: number
 [k: string]: unknown
}
export interface SendCoins {
 send_tokens: SendTokens
}
export interface SendTokens {
 amount: number
 currency: number
 to: number
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
