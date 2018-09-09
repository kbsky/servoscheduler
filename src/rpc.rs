use std::collections::HashMap;
use std::sync::{Arc,RwLock};

use server::*;

service! {
    rpc list_actuators() -> HashMap<u32, Actuator>;
    rpc get_schedule(actuator_id: u32) -> Schedule | Error;

    rpc set_default_state(actuator_id: u32, default_state: ActuatorState) -> () | Error;

    rpc add_time_slot(actuator_id: u32, time_period: TimePeriod, actuator_state: ActuatorState, enabled: bool) -> u32 | Error;
    rpc remove_time_slot(actuator_id: u32, time_slot_id: u32) -> () | Error;
    rpc time_slot_set_time_period(actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> () | Error;
    rpc time_slot_set_enabled(actuator_id: u32, time_slot_id: u32, enabled: bool) -> () | Error;
    rpc time_slot_set_actuator_state(actuator_id: u32, time_slot_id: u32, actuator_state: ActuatorState) -> () | Error;
    rpc time_slot_add_time_override(actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> u32 | Error;
    rpc time_slot_remove_time_override(actuator_id: u32, time_slot_id: u32, time_override_id: u32) -> () | Error;
}

#[derive(Clone)]
pub struct RpcServer {
    pub server: Arc<RwLock<Server>>,
}

impl RpcServer {
    pub fn new() -> RpcServer {
        RpcServer {
            server: Arc::new(RwLock::new(Server::new())),
        }
    }
}

type ResultNever<T> = ::std::result::Result<T, ::tarpc::util::Never>;

impl SyncService for RpcServer {
    fn list_actuators(&self) -> ResultNever<HashMap<u32, Actuator>> {
        Ok(self.server.read().unwrap().list_actuators().clone())
    }

    fn get_schedule(&self, actuator_id: u32) -> Result<Schedule> {
        self.server.read().unwrap().get_schedule(actuator_id).map(|s| s.clone())
    }

    fn set_default_state(&self, actuator_id: u32, default_state: ActuatorState) -> Result<()> {
        self.server.write().unwrap().set_default_state(actuator_id, default_state)
    }

    fn add_time_slot(&self, actuator_id: u32, time_period: TimePeriod, actuator_state: ActuatorState, enabled: bool) -> Result<u32> {
        self.server.write().unwrap().add_time_slot(actuator_id, time_period, actuator_state, enabled)
    }

    fn remove_time_slot(&self, actuator_id: u32, time_slot_id: u32) -> Result<()> {
        self.server.write().unwrap().remove_time_slot(actuator_id, time_slot_id)
    }

    fn time_slot_set_time_period(&self, actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> Result<()> {
        self.server.write().unwrap().time_slot_set_time_period(actuator_id, time_slot_id, time_period)
    }

    fn time_slot_set_enabled(&self, actuator_id: u32, time_slot_id: u32, enabled: bool) -> Result<()> {
        self.server.write().unwrap().time_slot_set_enabled(actuator_id, time_slot_id, enabled)
    }

    fn time_slot_set_actuator_state(&self, actuator_id: u32, time_slot_id: u32, actuator_state: ActuatorState) -> Result<()> {
        self.server.write().unwrap().time_slot_set_actuator_state(actuator_id, time_slot_id, actuator_state)
    }

    fn time_slot_add_time_override(&self, actuator_id: u32, time_slot_id: u32, time_period: TimePeriod) -> Result<u32> {
        self.server.write().unwrap().time_slot_add_time_override(actuator_id, time_slot_id, time_period)
    }

    fn time_slot_remove_time_override(&self, actuator_id: u32, time_slot_id: u32, time_override_id: u32) -> Result<()> {
        self.server.write().unwrap().time_slot_remove_time_override(actuator_id, time_slot_id, time_override_id)
    }
}

/* impl FutureService for RpcServer {
    type GetScheduleFut = Result<Schedule>;
    fn get_schedule(&self, actuator_id: u32) -> Self::GetScheduleFut {
        self.server.read().unwrap().get_schedule(actuator_id)
    }

    type SetDefaultStateFut = Result<()>;
    fn set_default_state(&self, actuator_id: u32, default_state: ActuatorState) -> Self::SetDefaultStateFut {
        self.server.write().unwrap().set_default_state(actuator_id, default_state)
    }

    type AddTimeSlotFut = Result<u32>;
    fn add_time_slot(&self,
                     actuator_id: u32,
                     time_period: TimePeriod,
                     actuator_state: ActuatorState,
                     enabled: bool) -> Self::AddTimeSlotFut {
        self.server.write().unwrap().add_time_slot(actuator_id, time_period, actuator_state, enabled)
    }
} */
