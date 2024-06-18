
use rand::Rng;
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use tcod::colors::*;
use serde::{Deserialize, Serialize};

use crate::object::{Object, Fighter, Transition, DeathCallback, from_dungeon_level, is_blocked};
use crate::monster_ai::Ai;
use crate::map::{Map, Rect};
use crate::game::{Tcod, Game};


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Monster {
	Orc,
	Troll,
	Bandit,
	Warrior,
	Zombie,
	Demon,
}

pub fn monster_table(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
    
    // max monsters based on level
    let max_monsters = from_dungeon_level(
        &[
            Transition { level: 1, value: 2 },
            Transition { level: 4, value: 3 },
            Transition { level: 6, value: 5 },
            Transition { level: 11, value: 2 },
            Transition { level: 15, value: 3 },
            Transition { level: 18, value: 5 },
            Transition { level: 21, value: 3 },
            Transition { level: 24, value: 4 },
            Transition { level: 27, value: 6 },
        ],
        level,
    );

    // get random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

	// orc chance random table
    let orc_chance = from_dungeon_level(
        &[
            Transition { level: 1, value: 100 },
            Transition { level: 3, value: 85 },
            Transition { level: 5, value: 70 },
            Transition { level: 7, value: 40 },
            Transition { level: 11, value: 0 },
        ],
        level,
    );

	// troll chance random table
    let troll_chance = from_dungeon_level(
        &[
            Transition { level: 3, value: 15 },
            Transition { level: 5, value: 30 },
            Transition { level: 7, value: 60 },
            Transition { level: 11, value: 50 },
            Transition { level: 14, value: 10 },
            Transition { level: 17, value: 0 },
        ],
        level,
    );

    // bandit chance random table
    let bandit_chance = from_dungeon_level(
        &[
            Transition { level: 11, value: 50 },
            Transition { level: 14, value: 60 },
            Transition { level: 17, value: 55 },
            Transition { level: 21, value: 10 },
        ],
        level,
    );

    // warrior chance random table
    let warrior_chance = from_dungeon_level(
        &[
            Transition { level: 14, value: 30 },
            Transition { level: 17, value: 45 },
            Transition { level: 21, value: 70 },
        ],
        level,
    );

    // monster random table
    let monster_chances = &mut [
        Weighted {
            weight: orc_chance,
            item: Monster::Orc,
        },
        Weighted {
            weight: troll_chance,
            item: Monster::Troll,
        },
        Weighted {
            weight: bandit_chance,
            item: Monster::Bandit,
        },
        Weighted {
            weight: warrior_chance,
            item: Monster::Warrior,
        },
    ];

    // create a weighted choice table from the chances
    let monster_choice = WeightedChoice::new(monster_chances);

    for _ in 0..num_monsters {

        // get random spot for monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        
        // select monster based on random sample from this level's weighted choice table
        let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {

            Monster::Orc => {
                // create orc
                let mut orc = Object::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter {
                    base_max_hp: 20,
                    hp: 20,
                    base_defense: 0,
                    base_power: 4,
                    base_magic: 0,
                    xp: 35,
                    on_death: DeathCallback::Monster,
                });
                orc.ai = Some(Ai::Basic);
                orc
            }
            Monster::Troll => {
                 // create troll 
                let mut troll = Object::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(Fighter {
                    base_max_hp: 30,
                    hp: 30,
                    base_defense: 2,
                    base_power: 8,
                    base_magic: 0,
                    xp: 100,
                    on_death: DeathCallback::Monster,
                });
                troll.ai = Some(Ai::Basic);
                troll
            }
            Monster::Bandit => {
                 // create bandit 
                let mut bandit = Object::new(x, y, 'B', "bandit", LIGHT_GREEN, true);
                bandit.fighter = Some(Fighter {
                    base_max_hp: 45,
                    hp: 45,
                    base_defense: 3,
                    base_power: 10,
                    base_magic: 0,
                    xp: 175,
                    on_death: DeathCallback::Monster,
                });
                bandit.ai = Some(Ai::Basic);
                bandit
            }
            Monster::Warrior => {
                 // create warrior 
                let mut warrior = Object::new(x, y, 'W', "warrior", WHITE, true);
                warrior.fighter = Some(Fighter {
                    base_max_hp: 60,
                    hp: 60,
                    base_defense: 5,
                    base_power: 12,
                    base_magic: 0,
                    xp: 250,
                    on_death: DeathCallback::Monster,
                });
                warrior.ai = Some(Ai::Basic);
                warrior
            }
            _ => unreachable!(),
        }; 
        
        // if this is a good spot, make monster alive and put in list so it will be placed 
        if !is_blocked(x, y, map, objects) {
            monster.alive = true;
            objects.push(monster);
        }
    }
}