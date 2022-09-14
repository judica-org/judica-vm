use super::instances::powerplant::PlantType;
use super::instances::powerplant::PowerPlant;
use super::BaseNFT;
use crate::entity::EntityID;
use crate::game::GameBoard;
use crate::util::Currency;
use crate::util::Price;

pub(crate) struct NFTMinter {}

impl NFTMinter {
    pub(crate) fn mint_power_plant(
        game: &mut GameBoard,
        // need to put a power plant price map somewhere
        resources: Vec<(Currency, Price)>,
        location: (u64, u64),
        plant_type: PlantType,
        owner: EntityID,
    ) {
        // check whether owner has enough of each material
        // there's a better way to do this
        let mut insufficient = false;
        for (currency, price) in resources {
            let token = &mut game.tokens[currency];
            token.transaction();
            if token.balance_check(&owner) < price {
                insufficient = true;
            }
            token.end_transaction();
        }
        if insufficient {
            return;
        }
        // create base nft?
        let base_power_plant = BaseNFT {
            owner,
            nft_id: game.alloc(),
            transfer_count: 0,
        };
        // insert into registry and get pointer
        let plant_ptr = game.nfts.add(Box::new(base_power_plant));
        // create PowerPlant nft
        let new_plant = PowerPlant::new(plant_ptr, plant_type, location);
        // add to plant register, need to return Plant?
        let _ = game.nfts.power_plants.insert(plant_ptr, new_plant).unwrap();

        // exchange (or burn?) tokens
        for (currency, price) in resources {
            let token = &mut game.tokens[currency];
            token.transaction();
            let _ = token.transfer(&owner, &plant_ptr.0, price);
            token.end_transaction();
        }
    }
}
