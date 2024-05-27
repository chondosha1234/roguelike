
use std::cmp;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use rand::Rng;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};  // rename tcod Map type as FovMap
use tcod::input::{self, Event, Key, Mouse};
use serde::{Deserialize, Serialize};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;   //default algorithm 
const FOV_LIGHT_WALLS: bool = true;  // light walls or not
const TORCH_RADIUS: i32 = 10;
const MAX_INVENTORY_SIZE: usize = 26;
const INVENTORY_WIDTH: i32 = 50;
const HEAL_AMOUNT: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const LIGHTNING_DAMAGE: i32 = 40;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3; 
const FIREBALL_DAMAGE: i32 = 25;
const PLAYER: usize = 0; // player will always be first object in list 
const LEVEL_UP_BASE: i32 = 200; // need 200 xp for first level up
const LEVEL_UP_FACTOR: i32 = 150; // increase needed xp per each lvl up
const LEVEL_SCREEN_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;

// size and coordinates for gui 
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;

// message gui constants
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const COLOR_DARK_WALL: Color = Color { r:0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const LIMIT_FPS: i32 = 20; // 20 fps maximum


// struct to hold all tcod related things for convenience in passing 
struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    key: Key,
    mouse: Mouse,
}


/*
 *  Map, Tile, Rect struct and implementations 
 */

type Map = Vec<Vec<Tile>>;  // 2d array of tiles 

#[derive(Serialize, Deserialize)]
struct Game {
    map: Map,
    messages: Messages,
    inventory: Vec<Object>,
    dungeon_level: u32,
}


// struct of map tile and properties
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Tile {
    blocked: bool,
    explored: bool,
    block_sight: bool,
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
struct Rect {
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
 *  Object struct, implementation, and related things
 */

// generic object: player, monster, item, stairs
#[derive(Debug, Serialize, Deserialize)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,  // can be some or none 
    ai: Option<Ai>,                         
    item: Option<Item>,
    always_visible: bool,
    level: i32,
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
            always_visible: false,
            level: 1,
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
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
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

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

// struct for level transitions 
struct Transition {
    level: u32,
    value: u32,
}

/*
 *  Components for objects
 */

// combat related properties and methods (player, npc, enemy)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    xp: i32,
    on_death: DeathCallback,
}

// item related properties and methods 
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
}

// use result for items 
enum UseResult {
    UsedUp,
    Cancelled,
}

// death callback function types 
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum DeathCallback {
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

// monster artificial intelligence
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

/*
 *  Message struct and implementation 
 */

// struct to hold list of messages -- each message has String for message and color 
#[derive(Serialize, Deserialize)]
struct Messages {
    messages: Vec<(String, Color)>,
}

impl Messages {

    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    // add new message as tuple 
    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color));
    }

    //Create a "Double Ended Iterator" over the messages
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }
}


/***********************************************************************************/


// function to create map with vec! macro 
fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
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
fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
    
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
                    max_hp: 20,
                    hp: 20,
                    defense: 0,
                    power: 4,
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
                    max_hp: 30,
                    hp: 30,
                    defense: 2,
                    power: 8,
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
            let dice = rand::random::<f32>();
            // create potion (70%)
            let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
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


/*************************************************************************************/


// move object by a given amount
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

// function to move to an object (usually monster toward player)
fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
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
fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for blocking objects
    // checks all objects and sees if in same spot and blocking, returns bool
    objects.iter().any(|object| object.blocks && object.pos() == (x, y))
}


fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
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

// function for player to pick up item 
fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
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
        game.inventory.push(item);
    }
}

// function to drop item from inventory to x/y of player
fn drop_item(inventory_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);

    game.messages.add(format!("You dropped a {}.", item.name), YELLOW);
    // item needs to be in list again to draw it
    objects.push(item);
}

fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    
    use Item::*;
    // call the 'use_function' if defined 
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
            Fireball => cast_fireball,
        };

        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                // destroy after use, unless cancelled
                game.inventory.remove(inventory_id);
            }
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

// function to cast heal 
fn cast_heal(_inventory_id: usize, _tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    // heal the player
    if let Some(fighter) = objects[PLAYER].fighter {
        // already at max health, can't use ability
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health!", RED);
            return UseResult::Cancelled;
        }
        // do the heal 
        game.messages.add("Your wounds start to heal!", LIGHT_VIOLET);
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    // the if let condition failed for some reason 
    UseResult::Cancelled
}

// function to use lightning attack on nearest enemy to player
fn cast_lightning(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    // find closest enemy inside max range
    let monster_id = closest_monster(tcod, objects, LIGHTNING_RANGE);
    if let Some(monster_id) = monster_id {
        // damage it with spell
        game.messages.add(
            format!("A lightning bolt strikes {}! Damage is {} hit points.", objects[monster_id].name, LIGHTNING_DAMAGE),
            LIGHT_BLUE,
        );
        if let Some(xp) = objects[monster_id].take_damage(LIGHTNING_DAMAGE, game) {
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        // no enemy found in range
        game.messages.add("No enemy close enough to strike!", RED);
        UseResult::Cancelled
    }
}

// function to use confuse ability
fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut[Object]) -> UseResult {
    
    game.messages.add("Left click an enemy to confuse it, or right click to cancel.", LIGHT_CYAN);

    // find closest enemy in range and confuse it
    let monster_id = target_monster(tcod, game, objects, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        // get old ai, make it Basic by default
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        // replace ai with confused 
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        
        game.messages.add(
            format!("The eyes of {} look vacant, as they start to stumble around", objects[monster_id].name),
            LIGHT_GREEN,
        );

        UseResult::UsedUp
    } else {
        // no enemy in the max range
        game.messages.add("No enemy is close enough to confuse.", RED);
        UseResult::Cancelled
    }
}

// function to cast targeted fireball 
fn cast_fireball(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    // ask player for target tile
    game.messages.add("Left click tile to target fireball, or Right click to cancel.", LIGHT_CYAN);

    let (x, y) = match target_tile(tcod, game, objects, None) {
        Some(tile_pos) => tile_pos,  // target_tile returned tuple, set it to (x, y)
        None => return UseResult::Cancelled,  // target_tile returned none, user cancelled
    };

    game.messages.add(
        format!("The fireball explodes, burning everything within {} tiles!", FIREBALL_RADIUS),
        ORANGE,
    );
    
    let mut xp_to_gain = 0;  // hold sum of xp from multiple targets
    // go through all objects and see if they are in blast radius 
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x, y) <= FIREBALL_RADIUS as f32 && obj.fighter.is_some() {
            game.messages.add(
                format!("The {} gets burned for {} hit points!", obj.name, FIREBALL_DAMAGE),
                ORANGE,
            );
            if let Some(xp) = obj.take_damage(FIREBALL_DAMAGE, game) {
                // don't give player xp from hitting themselves
                if id != PLAYER {
                    // add to sum of xp
                    xp_to_gain += xp;
                }
            }
        }
    }
    // now add sum to player xp
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    // return use result
    UseResult::UsedUp
}

// funtion to find the closest monster object to the player -- returns index of the monster
fn closest_monster(tcod: &Tcod, objects: &[Object], max_range: i32) -> Option<usize> {
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


fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
    use Ai::*;
    
    // take() removes the value and puts None, but it will be replaced by return from functions 
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, objects), // returns Basic for new_ai
            Confused {
                previous_ai,
                num_turns,
            } => ai_confused(monster_id, tcod, game, objects, previous_ai, num_turns),
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

// monster ai function to move and attack 
fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) -> Ai {
    // a basic monster takes its turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();

    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far 
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);

        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {  // checks if it is fighter
            // close enough to attack (if player is alive)
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game); 
        }
    }
    Ai::Basic
}

fn ai_confused(
    monster_id: usize, 
    _tcod: &Tcod, 
    game: &mut Game, 
    objects: &mut [Object],
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    
    if num_turns >= 0 {
        // still confused - move random direction and decrement turns
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2), // -1, 0 or 1 in x direction
            rand::thread_rng().gen_range(-1, 2), // -1, 0 or 1 in y direction
            &game.map,
            objects,
        );
        // return modified confused ai 
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        // end of effect, restore previous AI (this one will be deleted)
        game.messages.add(
            format!("The {} is no longer confused!", objects[monster_id].name),
            RED,
        );
        *previous_ai  // return value from Box<Ai>
    }

}


// function to split vector into 2 parts so you can borrow from 2 elements at the same time
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
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

fn level_up(tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
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
                    format!("Constitution (+20 HP, from {}", fighter.max_hp),
                    format!("Strength (+1 attack, from {}", fighter.power),
                    format!("Agility (+1  defense, from {}", fighter.defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        fighter.xp -= level_up_xp;

        match choice.unwrap() {
            0 => {
                fighter.max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.power += 1;
            }
            2 => {
                fighter.defense += 1;
            }
            _ => unreachable!(),
        }
    }
}


// returns value that depends on level
// table specifies what value occurs at each level, default is 0
fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}


/****************************************************************************************/


fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    // check number of options to inventory size
    let error_msg = format!("Cannot have a menu with more than {} options", MAX_INVENTORY_SIZE);
    assert!(
        options.len() <= MAX_INVENTORY_SIZE,
        "{}", error_msg
    );

    // calculate total height -- header plus options
    let header_height = if header.is_empty() {
        0    // blank header remove the empty line
    } else {
        root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header)
    };
    let height = options.len() as i32 + header_height;

    // create offscreen console for menu window
    let mut window = Offscreen::new(width, height);

    // print header with auto wrap
    window.set_default_foreground(WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
    );

    // print all the options 
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;  // convert index into character A-Z
        let text = format!("({}) {}", menu_letter, option_text.as_ref());

        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text,
        );
    }

    // blit contents of menu window onto screen
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(
        &window,
        (0, 0),
        (width, height),
        root,
        (x, y),
        1.0,    // foreground opacity
        0.7,    // background opacity
    );

    // present root console and wait for key press to continue
    root.flush();
    let key = root.wait_for_keypress(true);

    // convert the ASCII code to an index, and return option if valid
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

// make menu of inventory items as options
fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    // check case that inventory is empty
    let options = if inventory.len() == 0 {
        // add this string as an option to let user know inventory is empty
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // if item is chosen, return it
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

// main menu function
fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("../menu_background.png")
        .ok()
        .expect("Background image not found!");

    while !tcod.root.window_closed() {
        // show background image at twice normal console resolution
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));
        
        tcod.root.set_default_foreground(LIGHT_YELLOW);
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 4,
            BackgroundFlag::None,
            TextAlignment::Center,
            "Tombs of the Ancient Kings",
        );
        
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 2,
            BackgroundFlag::None,
            TextAlignment::Center,
            "Jonathan Miller -- Tutorial by tomassedovic",
        );

        //show options and wait for player choice
        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                // new game
                let (mut game, mut objects) = new_game(tcod);
                play_game(tcod, &mut game, &mut objects);
            }
            Some(1) => {
                // load game
                match load_game() {
                    Ok((mut game, mut objects)) => {
                        initialize_fov(tcod, &game.map);
                        play_game(tcod, &mut game, &mut objects);
                    }
                    Err(_e) => {
                        msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                // quit game 
                break;
            }
            _ => {}
        }
    }
}

// use menu function to display list of error messages 
fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}


/****************************************************************************************/


// function to draw all objects and map 
fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    
    // recompute fov if needed
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            // check if this position is in fov
            let visible = tcod.fov.is_in_fov(x, y);
            
            // wall is bool for block sight 
            let wall = game.map[x as usize][y as usize].block_sight;
            
            // set color based on fov and tile type 
            let color = match (visible, wall) {
                //out side field of view
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                // inside fov 
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };

            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                // if it is visible set explore 
                *explored = true;
            }

            if *explored {
                //show explored tiles only
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }
    
    // clone the objects into this mutable vector and filter out objects not in fov
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| {
            tcod.fov.is_in_fov(o.x, o.y)
                || (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
            })
        .collect();
    // sort so non blocking objects are first 
    to_draw.sort_by(|o1, o2| {o1.blocks.cmp(&o2.blocks) });

    // draw all objects in list 
    for object in &to_draw {
        //if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        //}
    }

    // blit contents of con to root console
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );  
 
    // prepare to render gui panel 
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // show player stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        LIGHT_RED,
        DARKER_RED,
    );
    
    // display dunegon level
    tcod.panel.print_ex(
        1,
        3,
        BackgroundFlag::None,
        TextAlignment::Left,
        format!("Dungeon Level: {}", game.dungeon_level),
    );

    // display names of objects under the mouse
    tcod.panel.set_default_foreground(LIGHT_GREY);
    tcod.panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(tcod.mouse, objects, &tcod.fov),
    );
 
    // print the game messages , one line at a time
    let mut y = MSG_HEIGHT as i32;
    // iterate through messages, most recent first (reverse) 
    for &(ref msg, color) in game.messages.iter().rev() {
        // get message height if word wrapped
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        // if y is out of bounds, then just stop adding more messages
        if y < 0 {
            break;
        }
        // print the message with right color at constant MSG_X and the calculated y
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
    
    // blit the contents of panel to root console
    blit(
        &tcod.panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        &mut tcod.root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );  

}

// function to render generic status bars -- can be hp, mana, exp, other things
fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color, 
) {
    // first calculate the width of the bar
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // render the background first
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    // now render bar on top 
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // add text last
    panel.set_default_foreground(WHITE);
    panel.print_ex(
        x + total_width / 2,
        y,
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

// return true means end game, return false means keep going 
fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {
    
    use tcod::input::KeyCode::*;
    use PlayerAction::*;
   
    let player_alive = objects[PLAYER].alive;

    match (tcod.key, tcod.key.text(), player_alive) {
        (
            Key { 
                code: Enter,
                alt: true,  // alt is true if alt is pressed too
                ..
            },
            _,    // text doesnt matter
            _,    // alive or dead doesnt matter 
        ) => {
            // alt + enter toggles fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn  // return player action 
        }

        (Key { code: Escape, ..}, _, _) => Exit,  // exit game, return player action exit
        // movement keys 
        (Key { code: Up, ..}, _, true) | (Key { code: NumPad8, ..}, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key { code: Down, ..}, _, true) | (Key { code: NumPad2, ..}, _, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        (Key { code: Left, ..}, _, true) | (Key { code: NumPad4, ..}, _, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key { code: Right, ..}, _, true) | (Key { code: NumPad6, ..}, _, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        (Key { code: Home, ..}, _, true) | (Key { code: NumPad7, ..}, _, true) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        }
        (Key { code: PageUp, ..}, _, true) | (Key { code: NumPad9, ..}, _, true) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        }
        (Key { code: End, ..}, _, true) | (Key { code: NumPad1, ..}, _, true) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        }
        (Key { code: PageDown, ..}, _, true) | (Key { code: NumPad3, ..}, _, true) => {
            player_move_or_attack(1, 1, game, objects);
            TookTurn
        }
        (Key { code: NumPad5, ..}, _, true) => {
            // do nothing -- wait for monster to come to you
            TookTurn
        }
        (Key { code: Text, ..}, "g", true) => {
            // pick up item
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "d", true) => {
            // show inventory, if item selected, drop it
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item you want to drop, or any other to cancel.\n",
                &mut tcod.root,
            );

            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "i", true) => {
            // show the inventory
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key shown next to an item to use it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, objects);
            }
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "c", true) => {
            // check player stats
            let player = &objects[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + level * LEVEL_UP_FACTOR;
            
            if let Some(fighter) = player.fighter.as_ref() {
                let msg = format!(
                        "Character Information

                Level: {}
                Experience: {}
                Next Level: {}

                Maximum HP: {}
                Attack: {}
                Defense: {}",
                    level, fighter.xp, level_up_xp, fighter.max_hp, fighter.power, fighter.defense
                );

                msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
            }
            
            DidntTakeTurn
        }
        (Key { code: Text, ..}, "<", true) => {
            // go down stairs, if player is on them
            let player_on_stairs = objects
                .iter()
                .any(|object| object.pos() == objects[PLAYER].pos() && object.name == "stairs");
            if player_on_stairs {
                next_level(tcod, game, objects);
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}

// return position of tile left clicked in players FOV
// returns (None, None) if right clicked
fn target_tile(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>,
) -> Option<(i32, i32)> {
    
    use tcod::input::KeyCode::Escape;

    loop {
        // render the screen - erases inventory and shows names under mouse
        tcod.root.flush();

        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => tcod.key = k,
            None => tcod.key = Default::default(),
        }
        render_all(tcod, game, objects, false);
        
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        // accept the target if in player fov and in designated range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        // left mouse pressed, in fov, and in range
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y));
        }
        
        // cancel if player clicks right button or escape
        if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
            return None; 
        }
    }    
}

// function to target specifically a monster instead of any tile
fn target_monster(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>,
) -> Option<usize> {   // return index of monster
    
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x, y)) => {
                // return the first clicked monster, keep looping until this
                for (id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id);
                    }
                }
            }
            None => return None,
        }
    }
}

// return a string with the name of all objects under mouse
fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
    // mouse cx and cy are coordinates of current mouse 
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    // create a list of names of all objects at the mouse coordinates in FOV
    let names = objects
        .iter()
        .filter(|obj| obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")  // join the names separated by commas, and return 
}


/******************************************************************************************/

fn new_game(tcod: &mut Tcod) -> (Game, Vec<Object>) {
    // create player object and object list 
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 100,
        hp: 100,
        defense: 1,
        power: 4,
        xp: 0,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![player];
    
    let mut game = Game {
        // generate map 
        map: make_map(&mut objects, 1),
        messages: Messages::new(),
        inventory: vec![],
        dungeon_level: 1,
    };
    
    initialize_fov(tcod, &game.map);

    // welcome message 
    game.messages.add(
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings!",
        RED,
    );
    // return tuple
    (game, objects)
 
}

// function to save game state
// return Ok or error - if game save fails 
fn save_game(game: &Game, objects: &[Object]) -> Result<(), Box<dyn Error>> {
    // convert game and object list to json
    let save_data = serde_json::to_string(&(game, objects))?;
    // create file names savegame
    let mut file = File::create("savegame")?;
    // write the json data to file
    file.write_all(save_data.as_bytes())?;
    // return Ok if successful
    Ok(())
}

// function to load saved game
fn load_game() -> Result<(Game, Vec<Object>), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Game, Vec<Object>)>(&json_save_state)?;
    Ok(result)
}

// function to handle initializing an FOV for new or loaded game
fn initialize_fov(tcod: &mut Tcod, map: &Map) {
    // populate FOV map according to generated map 
    for y in 0..MAP_HEIGHT{
        for x in 0..MAP_WIDTH {
            // tcod needs opposite values from what we set, so use negation
            tcod.fov.set(
                x,
                y,
                !map[x as usize][y as usize].block_sight,
                !map[x as usize][y as usize].blocked,
            );
        }
    }
    // unexplored areas start black (which is default background color)
    tcod.con.clear();
}

// function to handle main game loop 
fn play_game(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
   
    // force FOV "recompute" first time through game loop because invalid position
    let mut previous_player_position = (-1, -1);

    // main game loop 
    while !tcod.root.window_closed() {
        // clear screen of previous frame
        tcod.con.clear();
        
        // check for mouse events
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }
 
        // recompute if player has moved
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(tcod, game, objects, fov_recompute);
        
        tcod.root.flush();
        
        // level up check
        level_up(tcod, game, objects);

        // handle keys and exit game if needed
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(tcod, game, objects);
        if player_action == PlayerAction::Exit {
            save_game(game, objects).unwrap();
            break;
        }

        // let monsters take their turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                // if object has ai 
                if objects[id].ai.is_some() {
                    ai_take_turn(id, tcod, game, objects);
                }
            }
        }
    }
}

// move to next level 
fn next_level(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
    
    game.messages.add("You take a moment to rest and recover your strength.", VIOLET);
    // player rests and heals 50% 
    let heal_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp / 2);
    objects[PLAYER].heal(heal_hp);

    game.messages.add("After a moment of rest, you venture deeper into the dungeon...", RED);
    
    // add level and make new map and fov map
    game.dungeon_level += 1;
    // remove all objects except player 
    // note: player must be first element 
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _); // compare ptrs to object
    objects.truncate(1);
   
    game.map = make_map(objects, game.dungeon_level);
    initialize_fov(tcod, &game.map);
}


fn main() {

    tcod::system::set_fps(LIMIT_FPS);
    
    // root initialization 
    let root = Root::initializer()
        .font("../arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    // initialize console and fovmap in struct init 
    let mut tcod = Tcod { 
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(MAP_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT), 
        key: Default::default(),
        mouse: Default::default(),
    };
    
    main_menu(&mut tcod);
}
