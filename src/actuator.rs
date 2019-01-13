use std::collections::BTreeMap;
use std::fmt;
use std::num;
use std::result;
use std::str;
use std::sync::{Arc, RwLock};

use time::*;
use time_slot::*;
use utils::*;

use rpc::InvalArgError as IAE;
use rpc::Error::*;
pub type Result<T> = result::Result<T, ::rpc::Error>;

#[derive(Clone, Serialize, Deserialize)]
pub enum ActuatorType {
    Toggle,
    FloatValue { min: f64, max: f64 },
}

impl fmt::Display for ActuatorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActuatorType::Toggle => write!(f, "Toggle"),
            ActuatorType::FloatValue { min, max } => write!(f, "Float [{}, {}]", min, max),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ActuatorState {
    Toggle(bool),
    FloatValue(f64),
}

impl fmt::Display for ActuatorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActuatorState::Toggle(value) => write!(f, "{}", if *value { "On" } else { "Off " }),
            ActuatorState::FloatValue(value) => write!(f, "{}", value),
        }
    }
}

impl str::FromStr for ActuatorState {
    type Err = num::ParseFloatError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "on" => Ok(ActuatorState::Toggle(true)),
            "off" => Ok(ActuatorState::Toggle(false)),
            _ => f64::from_str(s).map(|f| ActuatorState::FloatValue(f))
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActuatorInfo {
    pub name: String,
    pub actuator_type: ActuatorType,
}

impl ValidCheck for ActuatorInfo {
    fn valid(&self) -> bool {
        match self.actuator_type {
            ActuatorType::Toggle => true,
            ActuatorType::FloatValue { min, max } => min < max,
        }
    }
}

pub struct Actuator {
    pub info: ActuatorInfo,

    timeslots: BTreeMap<u32, TimeSlot>,
    default_state: ActuatorState,

    next_timeslot_id: u32,
    // TODO: would be nice to be per-timeslot, but shouldn't be exposed via RPC either...
    next_override_id: u32,
}
pub type ActuatorHandle = Arc<RwLock<Actuator>>;

impl Actuator {
    pub fn new(info: ActuatorInfo, default_state: ActuatorState) -> ActuatorHandle {
        let result_handle = Arc::new(RwLock::new(Actuator {
            info,
            timeslots: BTreeMap::new(),
            default_state,
            next_timeslot_id: 0,
            next_override_id: 0,
        }));

        result_handle
    }

    pub fn timeslots(&self) -> &BTreeMap<u32, TimeSlot> {
        &self.timeslots
    }

    pub fn default_state(&self) -> &ActuatorState {
        &self.default_state
    }

    pub fn set_default_state(&mut self, default_state: ActuatorState) -> Result<()> {
        if !self.valid_state(&default_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        self.default_state = default_state;
        Ok(())
    }

    pub fn add_time_slot(&mut self,
                         time_period: TimePeriod,
                         actuator_state: ActuatorState,
                         enabled: bool) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        if !self.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        // Check for overlaps.
        for (id, ts) in self.timeslots.iter() {
            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        // All good, insert the timeslot.
        let id = self.next_timeslot_id;
        self.timeslots.insert(id, TimeSlot::new(enabled, actuator_state, time_period));
        self.next_timeslot_id += 1;

        println!("Added time slot, len = {:?}", self.timeslots.len());

        Ok(id)
    }

    pub fn remove_time_slot(&mut self, time_slot_id: u32) -> Result<()> {
        if self.timeslots.remove(&time_slot_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(IAE::TimeSlotId))
        }
    }

    pub fn time_slot_set_time_period(&mut self, time_slot_id: u32,
                                     time_period: TimePeriod) -> Result<()> {
        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Result<&mut TimeSlot> = Err(InvalidArgument(IAE::TimeSlotId));
        for (id, ts) in self.timeslots.iter_mut() {
            if *id == time_slot_id {
                target_ts = Ok(ts);
                continue;
            }

            if ts.overlaps(&time_period) {
                target_ts = Err(TimeSlotOverlap(*id));
                break;
            }
        }

        let ts = target_ts?;

        // Update specified fields.
        let mut new_time_period = ts.time_period.clone();

        if time_period.time_interval.start != Time::EMPTY {
            new_time_period.time_interval.start = time_period.time_interval.start;
        }
        if time_period.time_interval.end != Time::EMPTY {
            new_time_period.time_interval.end = time_period.time_interval.end;
        }
        if time_period.date_range.start != Date::empty_date() {
            new_time_period.date_range.start = time_period.date_range.start;
        }
        if time_period.date_range.end != Date::empty_date() {
            new_time_period.date_range.end = time_period.date_range.end;
        }
        if !time_period.days.is_empty() {
            new_time_period.days = time_period.days;
        }

        // Check that the specified fields were valid.
        if !new_time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        // All good, modify the timeslot.
        ts.time_period = new_time_period;
        Ok(())
    }

    pub fn time_slot_set_enabled(&mut self, time_slot_id: u32,
                                 enabled: bool) -> Result<()> {
        let time_slot = self.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        time_slot.enabled = enabled;
        Ok(())
    }

    pub fn time_slot_set_actuator_state(&mut self, time_slot_id: u32,
                                        actuator_state: ActuatorState) -> Result<()> {
        if !self.valid_state(&actuator_state) {
            return Err(InvalidArgument(IAE::ActuatorState))
        }

        let time_slot = self.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        time_slot.actuator_state = actuator_state;
        Ok(())
    }

    pub fn time_slot_add_time_override(&mut self, time_slot_id: u32,
                                       time_period: TimePeriod) -> Result<u32> {
        if !time_period.valid() {
            return Err(InvalidArgument(IAE::TimePeriod))
        }

        // Find the matching timeslot and check for overlaps.
        let mut target_ts: Option<&mut TimeSlot> = None;
        for (id, ts) in self.timeslots.iter_mut() {
            if *id == time_slot_id {
                target_ts = Some(ts);
                continue;
            }

            if ts.overlaps(&time_period) {
                return Err(TimeSlotOverlap(*id))
            }
        }

        if let Some(ts) = target_ts {
            // Also check there is no overlap with other overrides. The requirement is stronger:
            // two overrides cannot apply to the same day (not just day and time).
            for (id, or) in ts.time_override.iter() {
                if or.overlaps_dates(&time_period) {
                    return Err(TimeOverrideOverlap(*id))
                }
            }

            // All good, add the override.
            let id = self.next_override_id;
            ts.time_override.insert(id, time_period);
            self.next_override_id += 1;

            Ok(id)
        } else {
            Err(InvalidArgument(IAE::TimeSlotId))
        }
    }

    pub fn time_slot_remove_time_override(&mut self, time_slot_id: u32,
                                          time_override_id: u32) -> Result<()> {
        let time_slot = self.timeslots.get_mut(&time_slot_id)
            .ok_or(InvalidArgument(IAE::TimeSlotId))?;

        if time_slot.time_override.remove(&time_override_id).is_some() {
            Ok(())
        } else {
            Err(InvalidArgument(IAE::TimeOverrideId))
        }
    }

    fn valid_state(&self, state: &ActuatorState) -> bool {
        match self.info.actuator_type {
            ActuatorType::Toggle => match state {
                &ActuatorState::Toggle(_) => true,
                _ => false,
            },
            ActuatorType::FloatValue { min, max } => match state {
                &ActuatorState::FloatValue(value) => (min <= value && value <= max),
                _ => false
            },
        }
    }
}

impl ValidCheck for Actuator {
    fn valid(&self) -> bool {
        self.info.valid() && self.valid_state(&self.default_state)
    }
}
