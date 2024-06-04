

use tcod::colors::*;

use crate::game::{Tcod, Game};
use crate::object::{Object, closest_monster};
use crate::monster_ai::Ai;
use crate::item::UseResult;
use crate::graphics::{target_tile, target_monster};

const HEAL_AMOUNT: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const LIGHTNING_DAMAGE: i32 = 40;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;
const FIREBALL_RADIUS: i32 = 3; 
const FIREBALL_DAMAGE: i32 = 25;
const PLAYER: usize = 0;

// function to cast heal 
pub fn cast_heal(_inventory_id: usize, _tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
    let player = &mut objects[PLAYER];
    // heal the player
    if let Some(fighter) = player.fighter {
        // already at max health, can't use ability
        if fighter.hp == player.max_hp(game) {
            game.messages.add("You are already at full health!", RED);
            return UseResult::Cancelled;
        }
        // do the heal 
        game.messages.add("Your wounds start to heal!", LIGHT_VIOLET);
        player.heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    // the if let condition failed for some reason 
    UseResult::Cancelled
}

// function to use lightning attack on nearest enemy to player
pub fn cast_lightning(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
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
pub fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut[Object]) -> UseResult {
    
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
pub fn cast_fireball(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) -> UseResult {
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