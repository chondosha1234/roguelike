
use std::cmp;
use rand::Rng;
use tcod::colors::*;
use serde::{Deserialize, Serialize};

use crate::item::{Item, Slot, Equipment};
use crate::object::{Object, Fighter, Transition, DeathCallback, from_dungeon_level, is_blocked};
use crate::monster_ai::Ai;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const PLAYER: usize = 0; // player will always be first object in list 

/*
 *  Map, Tile, Rect struct and implementations 
 */

pub type Map = Vec<Vec<Tile>>;  // 2d array of tiles 

// struct of map tile and properties
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub blocked: bool,
    pub explored: bool,
    pub block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            explored: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            explored: false,
            block_sight: true,
        }
    }
}

// rectangle on map representing a room, has coordinates of top left and bottom right
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    // create new rectangle with top left and dimensions 
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x+w,
            y2: y+h,
        }
    }

    // get the center of a rectangle room -- used for start of tunnel 
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    // function to check if rooms are overlapping 
    pub fn intersects_with(&self, other: &Rect) -> bool {
        // return true if room intersects with another 
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

/*
 *  Map related functions
 */

 // function to create map with vec! macro 
pub fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
    // fill map with wall tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    
    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height 
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of bounds of map 
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // check intersections with other rooms using closure 
        // any() will run on every element aborts if it encounters false 
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room)); 
        
        // if it is valid spot then create room
        if !failed {
            // add room by drawing the map tiles 
            create_room(new_room, &mut map);

            // place objects in room
            place_objects(new_room, &map, objects, level);
            
            // get center coordinates of room
            let (new_x, new_y) = new_room.center();

            // put player in room if its first room
            if rooms.is_empty() {
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                // else need to connect this room to previous room 
                // get previous room center 
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // flip coin -- get random bool value 
                if rand::random() {
                    // first do horizontal then vertical
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first do vertical then horizontal 
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            // add room to list 
            rooms.push(new_room);
        }
    }
    
    // create stairs at the center of last room 
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', "stairs", WHITE, false);
    stairs.always_visible = true;
    objects.push(stairs);

    map   // return the map 
}

// function to add room to map 
fn create_room(room: Rect, map: &mut Map) {
    // go through tiles in rectangle and make them passable
    // loops exclude first and last to make walls
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

// function to create horizontal tunnels 
fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    // min and max used if x1 > x2
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

// function to create vertical tunnels 
fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    // min and max used if y1 > y2
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}


// function to place objects in a room
pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
    
    use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
    
    let max_monsters = from_dungeon_level(
        &[
            Transition { level: 1, value: 2 },
            Transition { level: 4, value: 3 },
            Transition { level: 6, value: 5 },
        ],
        level,
    );

    // get random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);
    
    // troll chance random table
    let troll_chance = from_dungeon_level(
        &[
            Transition { level: 3, value: 15 },
            Transition { level: 5, value: 30 },
            Transition { level: 7, value: 60 },
        ],
        level,
    );

    // monster random table
    let monster_chances = &mut [
        Weighted {
            weight: 80,
            item: "orc",
        },
        Weighted {
            weight: troll_chance,
            item: "troll",
        }
    ];
    let monster_choice = WeightedChoice::new(monster_chances);

    for _ in 0..num_monsters {
        // get random spot for monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        
        // 80% chance for orc
        let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
            "orc" => {
                // create orc
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter {
                    base_max_hp: 20,
                    hp: 20,
                    base_defense: 0,
                    base_power: 4,
                    base_magic: 0,
                    xp: 35,
                    on_death: DeathCallback::Monster,
                });
                orc.ai = Some(Ai::Basic);
                orc
            }
            "troll" => {
                 // create troll 
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(Fighter {
                    base_max_hp: 30,
                    hp: 30,
                    base_defense: 2,
                    base_power: 8,
                    base_magic: 0,
                    xp: 100,
                    on_death: DeathCallback::Monster,
                });
                troll.ai = Some(Ai::Basic);
                troll
            }
            _ => unreachable!(),
        }; 
        
        if !is_blocked(x, y, map, objects) {
            monster.alive = true;
            objects.push(monster);
        }
    }

    // max number of items per room
    let max_items = from_dungeon_level(
        &[
            Transition { level: 1, value: 1 },
            Transition { level: 4, value: 2 },
        ],
        level,
    );

    // get random number of items 
    let num_items = rand::thread_rng().gen_range(0, max_items + 1);

    // item random table
    let item_chances = &mut [
        Weighted {
            weight: from_dungeon_level(
                        &[Transition { level: 4, value: 5 }],
                        level,
                    ),
            item: Item::Sword,
        },
        Weighted {
            weight: from_dungeon_level(
                        &[Transition { level: 8, value: 15 }],
                        level,
                    ),
            item: Item::Shield,
        },
        Weighted {
            weight: 35,
            item: Item::Heal,
        },
        Weighted {
            weight: from_dungeon_level(
                        &[Transition { level: 4, value: 25 }],
                        level,
                    ),
            item: Item::Lightning,
        },
        Weighted {
            weight: from_dungeon_level(
                        &[Transition { level: 6, value: 25 }],
                        level,
                    ),
            item: Item::Fireball,
        },
        Weighted {
            weight: from_dungeon_level(
                        &[Transition { level: 2, value: 10 }],
                        level,
                    ),
            item: Item::Confuse,
        },
    ];
    let item_choice = WeightedChoice::new(item_chances);

    for _ in 0..num_items {
        // choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // only place if not blocked 
        if !is_blocked(x, y, map, objects) {
            // create potion (70%)
            let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                Item::Sword => {
                    // create a sword
                    let mut object = Object::new(x, y, '/', "sword", CYAN, false);
                    object.item = Some(Item::Sword);
                    object.equipment = Some(Equipment { 
                        equipped: false, 
                        slot: Slot::RightHand,
                        max_hp_bonus: 0,
                        power_bonus: 3,
                        defense_bonus: 0,
                        magic_bonus: 0,
                    });
                    object
                }
                Item::Shield => {
                    let mut object = Object::new(x, y, '[', "shield", DARKER_ORANGE, false);
                    object.item = Some(Item::Shield);
                    object.equipment = Some(Equipment { 
                        equipped: false, 
                        slot: Slot::LeftHand, 
                        max_hp_bonus: 0,
                        power_bonus: 0,
                        defense_bonus: 1,
                        magic_bonus: 0,
                    });
                    object
                }
                Item::Heal => {
                    // create healing potion 
                    let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
                    object.item = Some(Item::Heal);
                    object    
                }
                Item::Lightning => {
                    // create lightning bolt scroll (10%)
                    let mut object = Object::new(
                        x,
                        y,
                        '#',
                        "scroll of lightning bolt",
                        LIGHT_YELLOW,
                        false,
                    );
                    object.item = Some(Item::Lightning);
                    object
                }
                Item::Fireball => {
                    // create fireball scroll (10%)
                    let mut object = Object::new(
                        x,
                        y,
                        '#',
                        "scroll of fireball",
                        ORANGE,
                        false,
                    );
                    object.item = Some(Item::Fireball);
                    object
                }
                Item::Confuse => {
                    // create confuse scroll (10%) 
                    let mut object = Object::new(
                        x,
                        y,
                        '#',
                        "scroll of confusion",
                        PINK,
                        false,
                    );
                    object.item = Some(Item::Confuse);
                    object
                }
            };
 
            item.always_visible = true;
            objects.push(item);
        }
    }
}