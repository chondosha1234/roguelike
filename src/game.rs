
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};  // rename tcod Map type as FovMap
use tcod::input::{self, Event, Key, Mouse};
use serde::{Deserialize, Serialize};

use crate::message::Messages;
use crate::map::{Map, make_map};
use crate::object::{Object, PlayerAction, Fighter, DeathCallback, level_up};
use crate::item::*;
use crate::monster_ai::{Ai, ai_take_turn};
use crate::menu::{main_menu};
use crate::graphics::{render_all, handle_keys};

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;
const PLAYER: usize = 0;


// struct to hold all tcod related things for convenience in passing 
pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse,
}


#[derive(Serialize, Deserialize)]
pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
    pub dungeon_level: u32,
}


pub fn new_game(tcod: &mut Tcod) -> (Game, Vec<Object>) {
    // create player object and object list 
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 100,
        hp: 100,
        base_defense: 1,
        base_power: 2,
        base_magic: 0,
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

    // initial equipment
    let mut dagger = Object::new(0, 0, '-', "dagger", CYAN, false);
    dagger.item = Some(Item::Sword);
    dagger.equipment = Some(Equipment {
        equipped: true,
        slot: Slot::LeftHand,
        max_hp_bonus: 0,
        power_bonus: 2,
        defense_bonus: 0,
        magic_bonus: 0,
    });
    game.inventory.push(dagger);
    
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
pub fn save_game(game: &Game, objects: &[Object]) -> Result<(), Box<dyn Error>> {
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
pub fn load_game() -> Result<(Game, Vec<Object>), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Game, Vec<Object>)>(&json_save_state)?;
    Ok(result)
}

// function to handle initializing an FOV for new or loaded game
pub fn initialize_fov(tcod: &mut Tcod, map: &Map) {
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
pub fn play_game(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
   
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
pub fn next_level(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) {
    
    game.messages.add("You take a moment to rest and recover your strength.", VIOLET);
    // player rests and heals 50% 
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);

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
