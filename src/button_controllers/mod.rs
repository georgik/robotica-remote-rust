pub mod lights;
pub mod switch;

use std::convert::TryFrom;
use std::convert::TryInto;
use std::{collections::HashMap, ops::Sub};

use epd_waveshare::graphics::Display;
use ili9341::DisplaySize;
use log::*;

use crate::button::Button;

type Label = u32;

pub struct Command {
    location: String,
    device: String,
    message: serde_json::Value,
}

impl Command {
    pub fn get_topic(&self) -> String {
        format!("command/{}/{}", self.location, self.device)
    }

    pub fn get_message(&self) -> String {
        self.message.to_string()
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum Action {
    TurnOn,
    TurnOff,
    Toggle,
}

pub struct Subscription {
    pub topic: String,
    // pub msg_type: MessageType,
    pub label: Label,
}

#[allow(dead_code)]
pub enum DisplayState {
    HardOff,
    Error,
    Unknown,
    On,
    Off,
    OnOther,
}

pub trait Config {
    fn create_controller(&self) -> Box<dyn Controller>;
}

#[derive(Clone)]
pub enum Icon {
    Light,
    Fan,
}
#[derive(Clone)]
pub struct CommonConfig {
    pub name: String,
    pub location: String,
    pub device: String,
    pub action: Action,
    pub icon: Icon,
}

pub trait Controller {
    fn get_subscriptions(&self) -> Vec<Subscription>;
    fn process_disconnected(&mut self);
    fn process_message(&mut self, label: Label, data: String);
    fn get_display_state(&self) -> DisplayState;
    fn get_press_commands(&self) -> Vec<Command>;
    fn get_icon(&self) -> Icon;
}