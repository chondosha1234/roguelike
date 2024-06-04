
use tcod::colors::*;
use tcod::console::*;

use crate::game::{Tcod, Game, new_game, play_game, save_game, load_game, initialize_fov};
use crate::object::Object;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const INVENTORY_WIDTH: i32 = 50;
const MAX_INVENTORY_SIZE: usize = 26;
// size and coordinates for gui 
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
// message gui constants
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
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
pub fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    // check case that inventory is empty
    let options = if inventory.len() == 0 {
        // add this string as an option to let user know inventory is empty
        vec!["Inventory is empty.".into()]
    } else {
        inventory
            .iter()
            .map(|item| {
                // show additional info if item equipped
                match item.equipment {
                    Some(equipment) if equipment.equipped => {
                        format!("{} (on {})", item.name, equipment.slot)
                    }
                    _ => item.name.clone(),
                }
            })
            .collect()
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
pub fn main_menu(tcod: &mut Tcod) {
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
pub fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}