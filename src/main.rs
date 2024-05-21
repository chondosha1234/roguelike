use std::cmp;
use rand::Rng;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};  // rename tcod Map type as FovMap

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;   //default algorithm 
const FOV_LIGHT_WALLS: bool = true;  // light walls or not
const TORCH_RADIUS: i32 = 10;
const MAX_ROOM_MONSTERS: i32 = 3;
const PLAYER: usize = 0; // player will always be first object in list 

const COLOR_DARK_WALL: Color = Color { r:0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const LIMIT_FPS: i32 = 20; // 20 fps maximum

// struct to hold all tcod related things for convenience in passing 
struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

// struct of map tile and properties
#[derive(Clone, Copy, Debug)]
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

type Map = Vec<Vec<Tile>>;  // 2d array of tiles 

struct Game {
    map: Map,
}

// generic object: player, monster, item, stairs
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
        Object { x, y, char, color }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    // move by a given amount
    pub fn move_by(&mut self, dx: i32, dy: i32, game: &Game) {
        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    // set color and then draw character for object 
    pub fn draw(&self, con: &mut dyn Console) {       // Console is a trait -- dyn highlights this
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
}




// function to create map with vec! macro 
fn make_map(objects: &mut Vec<Object>) -> Map {
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
            place_objects(new_room, objects);
            
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
fn place_objects(room: Rect, objects: &mut Vec<Object>) {
    // get random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // get random spot for monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        
        // 80% chance for orc
        let mut monster = if rand::random::<f32>() < 0.8 {
            // create orc
            Object::new(x, y, 'o', DESATURATED_GREEN)
        } else {
            // create troll 
            Object::new(x, y, 'T', DARKER_GREEN)
        };

        objects.push(monster);
    }
}

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
    
    // draw all objects in list 
    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
    }


    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );  
}

// return true means end game, return false means keep going 
fn handle_keys(tcod: &mut Tcod, game: &Game, player: &mut Object) -> bool {
    
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
   
    let key = tcod.root.wait_for_keypress(true);   // returns type tcod::input::Key

    match key {
        Key { 
            code: Enter,
            alt: true,  // alt is true if alt is pressed too
            ..
        } => {
            // alt + enter toggles fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        },
        Key { code: Escape, ..} => return true,  // exit game
        // movement keys 
        Key { code: Up, ..} => player.move_by(0, -1, game),
        Key { code: Down, ..} => player.move_by(0, 1, game),
        Key { code: Left, ..} => player.move_by(-1, 0, game),
        Key { code: Right, ..} => player.move_by(1, 0, game),
        _ => {}
    }
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

    // initialize console and fovmap in struct init 
    let mut tcod = Tcod { 
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT), 
    };

    tcod::system::set_fps(LIMIT_FPS);
    
    // create player object and object list 
    let player = Object::new(0, 0, '@', WHITE);
    let mut objects = vec![player];
    
    let mut game = Game {
        // generate map 
        map: make_map(&mut objects),
    };
    
    // populate FOV map according to generated map 
    for y in 0..MAP_HEIGHT{
        for x in 0..MAP_WIDTH {
            // tcod needs opposite values from what we set, so use negation
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    // force FOV "recompute" first time through game loop because invalid position
    let previous_player_position = (-1, -1);

    // main game loop 
    while !tcod.root.window_closed() {
        tcod.con.clear();
        
        // recompute if player has moved
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();
        
        let player = &mut objects[PLAYER];
        let exit = handle_keys(&mut tcod, &game, player);
        if exit {
            break;
        }
    }
}
