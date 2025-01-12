// Adapted from https://github.com/iamabetterdogtht/esp32-touch-sensor-example/blob/a45cd34c43963305bd84fd7dbc31414a5e4c41f4/src/touch.rs

use esp_idf_svc::notify::Configuration;
use esp_idf_svc::notify::EspNotify;
use esp_idf_svc::notify::EspSubscription;
use std::ffi::c_void;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use arr_macro::arr;

use log::*;

use anyhow::Result;
use esp_idf_hal::gpio;
use esp_idf_sys as sys;
use esp_idf_sys::esp;

use embedded_hal::digital::blocking::InputPin;
use embedded_hal::digital::ErrorType;

use embedded_svc::event_bus::EventBus;
use embedded_svc::event_bus::Postbox;

use crate::input::{InputPinNotify, Value};

const NUM_TOUCH_PINS: usize = 10;

pub struct TouchControllerBuilder {
    touch_pins: [bool; NUM_TOUCH_PINS],
}

pub struct TouchPin {
    channel: sys::touch_pad_t,
    pin_number: i32,
    threshold: u16,
}

impl TouchControllerBuilder {
    pub fn new() -> Result<Self> {
        esp!(unsafe { sys::touch_pad_init() })?;
        esp!(unsafe { sys::touch_pad_set_fsm_mode(sys::touch_fsm_mode_t_TOUCH_FSM_MODE_TIMER) })?;
        esp!(unsafe {
            sys::touch_pad_set_voltage(
                sys::touch_high_volt_t_TOUCH_HVOLT_2V7,
                sys::touch_low_volt_t_TOUCH_LVOLT_0V5,
                sys::touch_volt_atten_t_TOUCH_HVOLT_ATTEN_1V,
            )
        })?;
        Ok(Self {
            touch_pins: [false; NUM_TOUCH_PINS],
        })
    }

    pub fn add_pin(&mut self, pin: impl gpio::TouchPin, threshold: u16) -> Result<TouchPin> {
        let channel = pin.touch_channel();
        self.touch_pins[channel as usize] = true;
        esp!(unsafe { sys::touch_pad_config(channel, 0) })?;
        esp!(unsafe { sys::touch_pad_set_thresh(channel, threshold) })?;
        // esp!(unsafe { sys::touch_pad_set_trigger_mode()});
        Ok(TouchPin {
            channel,
            pin_number: pin.pin(),
            threshold,
        })
    }
}

#[no_mangle]
#[inline(never)]
// #[link_section = ".iram1"]
unsafe extern "C" fn touch_handler(data: *mut c_void) {
    let _pin_number = data as i32;

    let pad_intr = sys::touch_pad_get_status();
    if esp!(sys::touch_pad_clear_status()).is_ok() {
        for (channel, notify) in NOTIFY.iter_mut().enumerate() {
            if (pad_intr >> channel) & 1 == 1 {
                match notify {
                    Some(x) => {
                        x.post(&1, Some(Duration::from_secs(0))).unwrap();
                    }
                    None => {}
                }
            }
        }
    }
}

impl TouchPin {
    pub fn read(&self) -> Result<u16> {
        let mut touch_value = 0;
        esp!(unsafe { sys::touch_pad_read_filtered(self.channel, &mut touch_value) })?;

        Ok(touch_value)
    }
}

impl ErrorType for TouchPin {
    type Error = anyhow::Error;
}

impl InputPin for TouchPin {
    fn is_high(&self) -> Result<bool> {
        Ok(self.read()? > self.threshold)
    }

    fn is_low(&self) -> Result<bool> {
        Ok(!self.is_high()?)
    }
}

impl InputPinNotify for TouchPin {
    fn subscribe<F: Fn(crate::input::Value) + Send + 'static>(&self, callback: F) {
        info!("About to start a background touch event loop");

        if unsafe { !INITIALIZED.load(Ordering::SeqCst) } {
            self.initialize();
            unsafe { INITIALIZED.store(true, Ordering::SeqCst) };
        }

        let config = Configuration::default();
        let mut notify = EspNotify::new(&config).unwrap();

        let s = notify
            .subscribe(move |v| {
                let v: Value = if *v != 0 { Value::High } else { Value::Low };
                callback(v);
            })
            .unwrap();

        unsafe {
            NOTIFY[self.channel as usize] = Some(notify);
            SUBSCRIPTION[self.channel as usize] = Some(s);
        }

        let state_ptr: *mut c_void = self.pin_number as *mut c_void;
        unsafe {
            esp!(sys::touch_pad_isr_register(Some(touch_handler), state_ptr)).unwrap();
        }
    }
}

impl TouchPin {
    fn initialize(&self) {
        unsafe {
            esp!(sys::touch_pad_filter_start(10)).unwrap();
            esp!(sys::touch_pad_clear_status()).unwrap();
            esp!(sys::touch_pad_intr_enable()).unwrap();
        }
    }
}

static mut INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut NOTIFY: [Option<EspNotify>; NUM_TOUCH_PINS] = arr![None; 10];
static mut SUBSCRIPTION: [Option<EspSubscription>; NUM_TOUCH_PINS] = arr![None; 10];
