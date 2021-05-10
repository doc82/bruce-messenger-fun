// use std::ptr;
use uuid::Uuid;

use crate::model::message::Message;

const MAX_RECENT_MESSAGE_LENGTH: u16 = 100;

// TODO: eventually we will place DB hooks in instead of storing only in-memory

#[derive(Default, Clone)]
pub struct Channel {
    pub messages: Vec<Message>,
    pub id: Uuid,
    pub game_id: Uuid,
    pub name: String,
}

impl Channel {
    pub fn new(override_id: Uuid, channel_name: &str, game_id: Uuid) -> Self {
        // For now we allow the ability to define the UUID instead of getting a randomly assigned one
        // With great power comes great responsibility... 
        let mut the_uuid = override_id;
        if override_id.is_nil() {
            the_uuid = Uuid::new_v4()
        }
        Channel {
            id: the_uuid,
            name: String::from(channel_name),
            game_id: game_id,
            messages: Vec::new(),
        }
    }

    pub fn messages_iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }

    // Get back a slice of the messages
    pub fn get_messages_paginate(&self, pos: usize, limit: usize) -> Vec<Message> {
        let mut nl = limit;
        if nl > MAX_RECENT_MESSAGE_LENGTH.into() {
            nl = MAX_RECENT_MESSAGE_LENGTH.into();
        }

        let total_messages = self.messages.len();
        if nl > total_messages {
            nl = total_messages;
        }

        return self.messages[pos..nl].to_vec();
    }

    // Assumes messages are already sorted by created date...
    pub fn get_recent_messages(&self) -> Vec<Message> {
        let mut total = 100;
        let total_messages = self.messages.len();
        if total_messages <= 0 {
            return self.messages.clone();
        } else {
            if total > total_messages {
                total = total_messages - 1;
            }
            return self.messages[0..total].to_vec();
        }
    }

    pub fn message_add(&mut self, message: Message) {
        self.messages.push(message);
        // hopefully we only ever sort after we add a new message!
        return self.message_sort_by_key();
    }

    // TODO: Do we need this to be public? We should only need this when we add an ew message (see add_message)
    pub fn message_sort_by_key(&mut self) {
        return self.messages.sort_by_key(|message| message.created_at);
    }

    // Get a Message by UUID (returns a reference, not an index)
    pub fn message_get_by_id(&mut self, messageId: Uuid) -> &Message {
        return self
            .messages
            .iter()
            .find(|&message| message.id == messageId)
            .unwrap();
    }

    // Edit a message
    pub fn message_edit_by_id(&mut self, messageId: Uuid) -> &Message {
        let message = self.message_get_by_id(messageId);
        // TODO: implement the edit
        return message;
    }
}
