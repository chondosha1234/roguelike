
use std::cmp;
use rand::Rng;
use tcod::colors::*;
use serde::{Deserialize, Serialize};

use crate::object::{Object, move_by, move_towards, mut_two};
use crate::game::{Tcod, Game};

const PLAYER: usize = 0; 

// monster artificial intelligence
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}


pub fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
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
pub fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) -> Ai {
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
