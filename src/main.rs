mod message;
mod map;
mod item;
mod monster_ai;
mod object;
mod graphics;
mod menu;
mod magic;
mod game;

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
use crate::game::{Tcod, Game};

const SCREEN_WIDTH: i32 = 100;   // orig 80
const SCREEN_HEIGHT: i32 = 60;  // orig 50
const MAP_WIDTH: i32 = 100;       // orig 80
const MAP_HEIGHT: i32 = 53;      // orig 43
const ROOM_MAX_SIZE: i32 = 12;
const ROOM_MIN_SIZE: i32 = 8;
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




fn main() {

    tcod::system::set_fps(LIMIT_FPS);
    
    // root initialization 
    let root = Root::initializer()
        .font("../dejavu_wide16x16_gs_tc.png", FontLayout::Tcod)
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
