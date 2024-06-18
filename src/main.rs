mod message;
mod map;
mod item;
mod monster_ai;
mod object;
mod graphics;
mod menu;
mod magic;
mod game;
mod monster;

//use std::error::Error;
//use std::fs::File;
//use std::io::{Read, Write};
//use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};  // rename tcod Map type as FovMap
//use tcod::input::{self, Event, Key, Mouse};
//use serde::{Deserialize, Serialize};

//use crate::message::Messages;
//use crate::map::{Map, make_map};
//use crate::object::{Object, PlayerAction, Fighter, DeathCallback, level_up};
//use crate::item::*;
//use crate::monster_ai::{Ai, ai_take_turn};
use crate::menu::{main_menu};
//use crate::graphics::{render_all, handle_keys};
use crate::game::{Tcod, Game};

const SCREEN_WIDTH: i32 = 100;   // orig 80
const SCREEN_HEIGHT: i32 = 60;  // orig 50
const MAP_WIDTH: i32 = 100;       // orig 80
const MAP_HEIGHT: i32 = 53;      // orig 43

//const PLAYER: usize = 0; // player will always be first object in list 

const PANEL_HEIGHT: i32 = 7;

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
