
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use serde::{Deserialize, Serialize};

use crate::menu::menu;
use crate::map::Map;
use crate::item::{Equipment, Item};
use crate::message::Messages;
use crate::game::{Tcod, Game};
use crate::monster_ai::Ai;

const LEVEL_UP_BASE: i32 = 200; // need 200 xp for first level up
const LEVEL_UP_FACTOR: i32 = 150; // increase needed xp per each lvl up
const LEVEL_SCREEN_WIDTH: i32 = 40;
const PLAYER: usize = 0; // player will always be first object in list 

/*
 *  Object struct, implementation, and related things
 */

// generic object: player, monster, item, stairs
#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,  // can be some or none 
    pub ai: Option<Ai>,                         
    pub item: Option<Item>,
    pub equipment: Option<Equipment>,
    pub always_visible: bool,
    pub level: i32,
    pub poisoned: bool,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object { 
            x: x, 
            y: y, 
            char: char, 
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
            equipment: None,
            always_visible: false,
            level: 1,
            poisoned: false,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    
    // return distance to another object
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    // return distance to a coordinate
    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }

    // set color and then draw character for object 
    pub fn draw(&self, con: &mut dyn Console) {       // Console is a trait -- dyn highlights this
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    // function for any fighter object to take damage, returns xp when object dies
    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        // check for death, call death function
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None   
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // simple attack formula
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            // make target take damage
            game.messages.add(
                format!("{} attacks {} for {} damage!", self.name, target.name, damage),
                WHITE,
            );
            if let Some(xp) = target.take_damage(damage, game) {
                // give exp to player -- take dmg only returns Some if death happens
                self.fighter.as_mut().unwrap().xp += xp;
            }
        } else {
            game.messages.add(
                format!("{} attacks {} but it has no effect!", self.name, target.name),
                WHITE,
            );
        }
    }
    
    // calculate current attack power including equipment
    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        // add up all power bonus from equipped items
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.power_bonus)
            .sum();
        base_power + bonus
    }

    // calculate current defense including equipment
    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        // add up all defense bonus from equipment
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.defense_bonus)
            .sum();
        base_defense + bonus 
    }

    // calculate current max_hp including equipment
    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        // add up equipment bonus
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.max_hp_bonus)
            .sum();
        base_max_hp + bonus
    }

    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
            }
        }
    }

    pub fn equip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(format!("Can't equip {:?} because it is not an item.", self), RED);
            return;
        }

        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                messages.add(format!("Equipped {} on {}.", self.name, equipment.slot), LIGHT_GREEN);
            }
        } else {
            messages.add(format!("Can't equip {:?} because its not equipment.", self), RED);
        }
    }

    pub fn unequip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(format!("Can't unequip {:?} because it is not an item.", self), RED);
            return;
        }

        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                equipment.equipped = false;
                messages.add(format!("Unequipped {} on {}.", self.name, equipment.slot), LIGHT_YELLOW);
            }
        } else {
            messages.add(format!("Can't unequip {:?} because its not equipment.", self), RED);
        }
    }
    
    // return list of all currently equipped items
    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "player" {
            game.inventory
                .iter()
                .filter(|item| item.equipment.map_or(false, |e| e.equipped))
                .map(|item| item.equipment.unwrap())
                .collect()
        } else {
            // other objects have no equipment
            vec![]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

// struct for level transitions 
pub struct Transition {
    pub level: u32,
    pub value: u32,
}

/*
 *  Components for objects
 */

// combat related properties and methods (player, npc, enemy)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub base_max_hp: i32,
    pub hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub base_magic: i32,
    pub xp: i32,
    pub on_death: DeathCallback,
}


// death callback function types 
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    // self is enum DeathCallback, object is the object dying 
    fn callback(self, object: &mut Object, game: &mut Game) {
        use DeathCallback::*;
        // callback is function of this type and it matches to the enum type
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        // call the appropriate function
        callback(object, game);
    }
}


/*
 * Object related functions
 */

 // move object by a given amount
pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

// function to move to an object (usually monster toward player)
pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // vector from this object to target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    // distance = sqrt (x^2 + y^2)
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // normalize it to length 1 (preserve direction), then round it and
    // convert to integer so movement is restricted to map grid 
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

// function to check if a tile is blocked by an blocking object
pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for blocking objects
    // checks all objects and sees if in same spot and blocking, returns bool
    objects.iter().any(|object| object.blocks && object.pos() == (x, y))
}


pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    // coordinates player is moving too
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // try to find attackable object
    let target_id = objects.iter().position(|object| object.fighter.is_some() && object.pos() == (x, y));

    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}


// funtion to find the closest monster object to the player -- returns index of the monster
pub fn closest_monster(tcod: &Tcod, objects: &[Object], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with slightly more than max range
    
    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER) 
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y) 
        {
            // calculate distance between object and player 
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                // it is closer than previous closest so replace 
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    // return closest enemy or initial None value 
    closest_enemy
}


// function to split vector into 2 parts so you can borrow from 2 elements at the same time
pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    // make sure the indices aren't the same
    assert!(first_index != second_index);
    // get the index to split at
    let split_at_index = cmp::max(first_index, second_index);
    // get 2 slices of the vector
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    // if first is less, use that index, and use 0 index of second slice 
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}


/*
 *  Death callback functions 
 */

fn player_death(player: &mut Object, game: &mut Game) {
    // game ended 
    game.messages.add("You died!", RED);

    // transform player to corpse
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    // transform it into corpse, it also doesn't block anymore
    // can't be attacked or move 
    game.messages.add(
        format!("{} is dead! You gain {} experience", monster.name, monster.fighter.unwrap().xp), 
        ORANGE,
    );

    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;   // disables the attack functionality
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

/*
 * Level up and xp 
 */ 

pub fn level_up(tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // see if player has enough xp to level up
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        // level up 
        player.level += 1;

        game.messages.add(
            format!("Your skills have increased! You are now level {}!", player.level),
            YELLOW,
        );
        // increase player stats based on player choice
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            // keep asking until choice made
            choice = menu(
                "Level up! Choose skill to increase:\n",
                &[
                    format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {})", fighter.base_power),
                    format!("Agility (+1  defense, from {})", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        fighter.xp -= level_up_xp;

        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}


// returns value that depends on level
// table specifies what value occurs at each level, default is 0
pub fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}