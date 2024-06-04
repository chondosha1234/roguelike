
use tcod::colors::*;
use serde::{Deserialize, Serialize};

use crate::object::Object;
use crate::game::{Tcod, Game};
use crate::magic::{cast_heal, cast_confuse, cast_fireball, cast_lightning};

const MAX_INVENTORY_SIZE: usize = 26;
const PLAYER: usize = 0;

// item related properties and methods 
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
    Sword,
    //Bow,
    //Wand,
    Shield,
    //Helmet,
    //ChestPiece,
    //Legs,
    //Boots,
    //Gloves,
    //Cape,
    //Ring,
}

// use result for items 
pub enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

// struct for equipment component of object
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    pub slot: Slot,
    pub equipped: bool,
    pub max_hp_bonus: i32,
    pub power_bonus: i32,
    pub defense_bonus: i32,
    pub magic_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    LeftHand,
    RightHand,
    Head,
    Chest,
    Legs,
    Feet,
    Hands,
    Back,
    LeftFinger,
    RightFinger,
}

// implementing Display trait for Slot enum
impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::LeftHand => write!(f, "left hand"),
            Slot::RightHand => write!(f, "right hand"),
            Slot::Head => write!(f, "head"),
            Slot::Chest => write!(f, "chest"),
            Slot::Legs => write!(f, "legs"),
            Slot::Feet => write!(f, "feet"),
            Slot::Hands => write!(f, "hands"),
            Slot::Back => write!(f, "back"),
            Slot::LeftFinger => write!(f, "left finger"),
            Slot::RightFinger => write!(f, "right finger"),

        }
    }
}


/*
 * Item and equipment related functions 
 */ 

// function for player to pick up item 
pub fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    // if reached max inventory size
    if game.inventory.len() >= MAX_INVENTORY_SIZE {
        game.messages.add(
            format!("Your inventory is full! cannot pick up {}!", objects[object_id].name),
            RED,
        );
    } else {
        // take the object out of list and place in item 
        let item = objects.swap_remove(object_id);
        game.messages.add(
            format!("You picked up {}!", item.name),
            GREEN,
        );
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);
        
        // if slot is empty for equipment type, then auto equip
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.messages);
            }
        }
    }
}


// function to drop item from inventory to x/y of player
pub fn drop_item(inventory_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    // unequip item if it is equipped
    if item.equipment.is_some() {
        item.unequip(&mut game.messages);
    }

    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);

    game.messages.add(format!("You dropped a {}.", item.name), YELLOW);
    // item needs to be in list again to draw it
    objects.push(item);
}

pub fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    
    use Item::*;
    // call the 'use_function' if defined 
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
            Sword => toggle_equipment,
            Shield => toggle_equipment,
            Bow => toggle_equipment,
    		Wand => toggle_equipment,
    		Shield => toggle_equipment,
    		Helmet => toggle_equipment,
    		ChestPiece => toggle_equipment,
    		Legs => toggle_equipment,
    		Boots => toggle_equipment,
    		Gloves => toggle_equipment,
    		Cape => toggle_equipment,
    		Ring => toggle_equipment,
        };

        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                // destroy after use, unless cancelled
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => {} // do nothing
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(
            format!("The {} cannot be used!", game.inventory[inventory_id].name),
            WHITE
        );
    }
    
}

// function to equip / unequip items
fn toggle_equipment(inventory_id: usize, _tcod: &mut Tcod, game: &mut Game, _objects: &mut [Object]) -> UseResult {
    
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    
    if let Some(current) = get_equipped_in_slot(equipment.slot, &game.inventory) {
        game.inventory[current].unequip(&mut game.messages);
    }

    if equipment.equipped {
        game.inventory[inventory_id].unequip(&mut game.messages);
    } else {
        game.inventory[inventory_id].equip(&mut game.messages);
    }
    UseResult::UsedAndKept
}

// get current equipment in a slot -- return index in object list
fn get_equipped_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item
            .equipment
            .as_ref()
            .map_or(false, |e| e.equipped && e.slot == slot) 
            {
                return Some(inventory_id);     
            }
    }
    None
}
