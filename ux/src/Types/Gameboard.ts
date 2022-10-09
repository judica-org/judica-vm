import { BuyNFTs, MintPowerPlantNFT, PurchaseMaterialsThenMintPowerPlantNFT, RemoveTokens, SellNFTs, SendALoggedChatMessageToAllPlayers, SendCoins, TradeCoins } from "./GameMove"

/**
 * an EntityID is just a "pointer" we assign to all different types of things in our game, e.g. - Users - Token Contracts - NFTs - etc
 *
 * EntityIDs are global and unique within the game state
 */
export type EntityID = string
export type LogEvent =
   | (
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
     )
   | (
       | "NoSuchUser"
       | {
           GameIsFinished: FinishReason
         }
       | {
           MoveSanitizationError: SanitizationError
         }
       | {
           TradeRejected: TradeError
         }
     )
/**
 * Allocator which can assign IDs sequentially
 */
export type EntityIDAllocator = number
export type Heartbeat = []
export type FinishReason =
   | "TimeExpired"
   | {
       DominatingPlayer: EntityID
     }
 export type SanitizationError = string
 export type TradeError =
   | "MarketSlipped"
   | {
       InvalidTrade: string
     }
   | {
       InsufficientTokens: string
     }
export type PlantType = "Solar" | "Hydro" | "Flare"
export type TradingPairID = string
export type SteelVariety = "Structural"
export type JoinCode = string;

export interface EmittedAppState {
    available_sequencers: [string, GameSetup][]
    chat_log?: [number, EntityID, string][]
    db_connection?: [string, string | null] | null
    energy_exchange?: UXNFTSale[]
    game_board?: GameBoard
    game_host_service?: GameHost | null
    host_key?: string
    materials_price_data?: UXMaterialsPriceData[]
    pending?: Pending | null
    power_plants?: UXPlantData[]
    signing_key: string
    super_handy_self_schema: object
    user_inventory?: UXUserInventory | null
    user_keys: string[]
    [k: string]: unknown
}
export interface GameSetup {
    finish_time?: number
    players: string[]
    start_amount: number
    [k: string]: unknown
}
export interface UXNFTSale {
    currency: EntityID
    nft_id: EntityID
    price: number
    seller: EntityID
    transfer_count: number
    [k: string]: unknown
}
/**
 * GameBoard holds the entire state of the game.
 */
export interface GameBoard {
    alloc: EntityIDAllocator
    asic_token_id: EntityID
    /**
     * If init = true, must be Some
     */
    bitcoin_token_id: EntityID
    callbacks: CallbackRegistry
    chat: [number, EntityID, string][]
    chat_counter: number
    /**
     * If init = true, must be Some
     */
    concrete_token_id: EntityID
    elapsed_time: number
    event_log: [number, LogEvent][]
    event_log_counter: number
    finish_time: number
    mining_subsidy: number
    nft_sales: NFTSaleRegistry
    nfts: NFTRegistry
    plant_prices: {
        [k: string]: [EntityID, number][]
    }
    player_move_sequence: {
        [k: string]: number
    }
    /**
     * If init = true, must be Some
     */
    real_sats_token_id: EntityID
    root_user: EntityID
    /**
     * If init = true, must be Some
     */
    silicon_token_id: EntityID
    /**
     * If init = true, must be Some
     */
    steel_token_id: EntityID
    swap: ConstantFunctionMarketMaker
    ticks: {
        [k: string]: Tick
    }
    tokens: TokenRegistry
    /**
     * Make this a vote over the map of users to current vote and let the turn count be dynamic
     */
    turn_count: number
    users: {
        [k: string]: UserData
    }
    users_by_key: {
        [k: string]: EntityID
    }
    [k: string]: unknown
}
/**
 * The registry of events. Events are processed in linear time order, then secondarily the order they are recieved
 */
export interface CallbackRegistry {
    /**
     * the key in this type is a virtual "time" at which the event should be removed and processed
     */
    callbacks: {
        [k: string]: string[]
    }
    [k: string]: unknown
}
/**
 * A Registry of all pending sales
 */
export interface NFTSaleRegistry {
    nfts: {
        [k: string]: NFTSale
    }
    [k: string]: unknown
}
/**
 * Represents an offer to sell an NFT
 */
export interface NFTSale {
    /**
     * The Currency the owner will be paid in
     */
    currency: EntityID
    /**
     * The Price the owner will accept
     */
    price: number
    /**
     * The seller's ID _at the time the sale was opened_, for replay protection
     */
    seller: EntityID
    /**
     * The transfer_count of the NFT _at the time the sale was opened_, for replay protection
     */
    transfer_count: number
    [k: string]: unknown
}
/**
 * A Registry of all NFTs and their MetaData
 */
export interface NFTRegistry {
    nfts: {
        [k: string]: unknown
    }
    power_plants: {
        [k: string]: PowerPlant
    }
    [k: string]: unknown
}
export interface PowerPlant {
    coordinates: [number, number]
    id: EntityID
    plant_type: PlantType
    watts: number
    [k: string]: unknown
}
/**
 * Registry of all Market Pairs
 */
export interface ConstantFunctionMarketMaker {
    markets: {
        [k: string]: ConstantFunctionMarketMakerPair
    }
    [k: string]: unknown
}
/**
 * Data for a single trading pair (e.g. Apples to Oranges tokens)
 *
 * Pairs have a balance in Apples and Oranges, as well as a token that represents a fractional interest (unit / total) redemptive right of Apples : Oranges
 */
export interface ConstantFunctionMarketMakerPair {
    /**
     * The ID of this pair
     */
    id: EntityID
    /**
     * The ID of the LP Tokens for this pair
     */
    lp: EntityID
    /**
     * The trading pair, should be normalized here
     */
    pair: TradingPairID
    reserve_a: number
    reserve_b: number
    [k: string]: unknown
}
export interface Tick {
    elapsed: number
    first_time: number
    [k: string]: unknown
}
/**
 * Holds Tokens and metadata for custom token types
 */
export interface TokenRegistry {
    hashboards: {
        [k: string]: HashBoardData
    }
    silicon: {
        [k: string]: Silicon
    }
    steel: {
        [k: string]: Steel
    }
    tokens: {
        [k: string]: unknown
    }
    [k: string]: unknown
}
/**
 * Parameters for a given HashBoard type
 */
export interface HashBoardData {
    hash_per_watt: number
    reliability: number
    [k: string]: unknown
}
/**
 * Properties of Silicon
 */
export interface Silicon {
    weight_in_kg: number
    [k: string]: unknown
}
/**
 * Properties of Steel
 */
export interface Steel {
    variety: SteelVariety
    weight_in_kg: number
    [k: string]: unknown
}
export interface UserData {
    key: string
    [k: string]: unknown
}
export interface GameHost {
    port: number
    url: string
    [k: string]: unknown
}
/**
 * A struct for passing token qty information to the UX for price calculation
 */
export interface UXMaterialsPriceData {
    asset_a: string
    asset_b: string
    display_asset: string
    mkt_qty_a: number
    mkt_qty_b: number
    trading_pair: TradingPairID
    [k: string]: unknown
}
export interface Pending {
    join_code: JoinCode
    password?: JoinCode | null
    [k: string]: unknown
}
export interface UXPlantData {
    coordinates: [number, number]
    for_sale: boolean
    hashrate: number
    id: EntityID
    miners: number
    owner: EntityID
    plant_type: PlantType
    watts: number
    [k: string]: unknown
}
export interface UXUserInventory {
    user_power_plants: {
        [k: string]: UXPlantData
    }
    user_token_balances: [string, number][]
    [k: string]: unknown
}
