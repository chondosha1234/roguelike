
use tcod::colors::*;
use tcod::console::*;

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20; // 20 fps maximum


// struct to hold all tcod related things for convenience in passing 
struct Tcod {
    root: Root,
}


// return true means end game, return false means keep going 
fn handle_keys(tcod: &mut Tcod, player_x: &mut i32, player_y: &mut i32) -> bool {
    false
}


fn main() {
    
    // root initialization 
    let root = Root::initializer()
        .font("../arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let mut tcod = Tcod { root };

    tcod::system::set_fps(LIMIT_FPS);

    let mut player_x = SCREEN_WIDTH / 2;
    let mut player_y = SCREEN_HEIGHT / 2;
    
    // main game loop 
    while !tcod.root.window_closed() {
        tcod.root.set_default_foreground(WHITE);
        tcod.root.clear();
        tcod.root.put_char(1, 1, '@', BackgroundFlag::None);
        tcod.root.flush();
        tcod.root.wait_for_keypress(true);
    }
}
