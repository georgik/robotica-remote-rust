use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use smart_leds::RGB;
use smart_leds_trait::SmartLedsWrite;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use super::DisplayCommand;

fn display_thread(mut leds: Ws2812Esp32Rmt, rx: mpsc::Receiver<DisplayCommand>) {
    let color = RGB::from((1, 1, 1));
    let blank_color = RGB::from((0, 0, 0));

    let mut blank = false;
    let mut pixels: [RGB<u8>; 16] = [color; 16];
    let blank_pixels: [RGB<u8>; 16] = [blank_color; 16];

    let iter = pixels.iter().copied();
    leds.write(iter).unwrap();

    for received in rx {
        match received {
            DisplayCommand::DisplayState(state, _icon, id, _name) => {
                let list_leds_or_none = match id {
                    2 => Some([14, 15, 0, 1]),
                    0 => Some([2, 3, 4, 5]),
                    1 => Some([6, 7, 8, 9]),
                    3 => Some([10, 11, 12, 13]),
                    _ => None,
                };

                let color = match state {
                    crate::button_controllers::DisplayState::HardOff => (0, 0, 0),
                    crate::button_controllers::DisplayState::Error => (1, 0, 0),
                    crate::button_controllers::DisplayState::Unknown => (1, 0, 0),
                    crate::button_controllers::DisplayState::On => (0, 1, 0),
                    crate::button_controllers::DisplayState::Off => (0, 0, 1),
                    crate::button_controllers::DisplayState::OnOther => (0, 1, 1),
                };

                let color = RGB::from(color);

                if let Some(list_leds) = list_leds_or_none {
                    for i in list_leds {
                        pixels[i] = color;
                    }

                    if !blank {
                        let iter = pixels.iter().copied();
                        leds.write(iter).unwrap();
                    }
                }
            }
            DisplayCommand::BlankAll => {
                blank = true;
                let iter = blank_pixels.iter().copied();
                leds.write(iter).unwrap();
            }
            DisplayCommand::UnBlankAll => {
                blank = false;
                let iter = pixels.iter().copied();
                leds.write(iter).unwrap();
            }
            DisplayCommand::ButtonPressed(_id) => {}
            DisplayCommand::ButtonReleased(_id) => {}
            DisplayCommand::Started => {}
            DisplayCommand::DisplayNone(_) => {}
            DisplayCommand::ShowPage(_) => {}
        }
    }
}

pub fn connect(pin: u32) -> Result<mpsc::Sender<DisplayCommand>> {
    let leds = Ws2812Esp32Rmt::new(0, pin).unwrap();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        display_thread(leds, rx);
    });

    Ok(tx)
}
