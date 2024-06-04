
use tcod::console::*;
use tcod::colors::*;
use tcod::input::{self, Event, Key, Mouse};
use tcod::map::{FovAlgorithm, Map as FovMap}; 

use crate::game::{Tcod, Game, next_level};
use crate::object::{Object, PlayerAction, player_move_or_attack};
use crate::menu::{inventory_menu, msgbox};
use crate::item::{pick_item_up, use_item, drop_item};

const PLAYER: usize = 0;
const LEVEL_UP_BASE: i32 = 200; // need 200 xp for first level up
const LEVEL_UP_FACTOR: i32 = 150; // increase needed xp per each lvl up

const SCREEN_WIDTH: i32 = 100;
const SCREEN_HEIGHT: i32 = 60;
const MAP_WIDTH: i32 = 100;
const MAP_HEIGHT: i32 = 53;
const LEVEL_SCREEN_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;   //default algorithm 
const FOV_LIGHT_WALLS: bool = true;  // light walls or not
const TORCH_RADIUS: i32 = 10;

const COLOR_DARK_WALL: Color = Color { r:0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

// function to draw all objects and map 
pub fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    
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
    let max_hp = objects[PLAYER].max_hp(game);
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
pub fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {
    
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
                    level, fighter.xp, level_up_xp, player.max_hp(game), player.power(game), player.defense(game),
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
pub fn target_tile(
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
pub fn target_monster(
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
