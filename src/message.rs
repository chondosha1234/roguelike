
use tcod::colors::*;
use serde::{Deserialize, Serialize};

/*
 *  Message struct and implementation 
 */

// struct to hold list of messages -- each message has String for message and color 
#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<(String, Color)>,
}

impl Messages {

    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    // add new message as tuple 
    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color));
    }

    //Create a "Double Ended Iterator" over the messages
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }
}
