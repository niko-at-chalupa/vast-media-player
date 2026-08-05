use slint::ComponentHandle;
use battery::units::ratio::percent;
use crate::{MainWindow, StatusBarData};
use std::{fmt::format, time::SystemTime};
use slint::SharedString;
use chrono::{DateTime, Local};

fn get_battery() -> Result<Option<battery::Battery>, battery::Error> {
    let manager = battery::Manager::new()?;

    for (_idx, maybe_battery) in manager.batteries()?.enumerate() {
        let battery = maybe_battery?;
        return Ok(Some(battery));
    }
    Ok(None)
}

pub fn populate_status_bar(ui: &MainWindow) {
    let battery = get_battery();
    if let Ok(Some(b)) = battery {
        ui.global::<StatusBarData>().set_has_battery(true);
        let battery_percentage = b.state_of_charge().get::<percent>().floor() as i32;
        ui.global::<StatusBarData>().set_battery(battery_percentage);
        match b.state() {
            battery::State::Charging => ui.global::<StatusBarData>().set_is_charging(true),
            battery::State::Discharging => ui.global::<StatusBarData>().set_is_charging(false),
            _ => ui.global::<StatusBarData>().set_is_charging(false),
        }
    } else {
        ui.global::<StatusBarData>().set_has_battery(false);
    }

    let system_time = SystemTime::now();
    let datetime: DateTime<Local> = system_time.into();
    let formatted_time = datetime.format("%I:%M %p").to_string();
    ui.global::<StatusBarData>().set_has_time(true);
    ui.global::<StatusBarData>().set_time(SharedString::from(formatted_time));
}