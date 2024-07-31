use shadow_ecs::ecs::system::schedule::{Phase, Schedule};

pub struct Init;
impl Phase for Init {}

pub struct Fixed;
impl Phase for Fixed {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Fixed>();
        schedule.add_schedule(Schedule::from::<PreFixedUpate>());
        schedule.add_schedule(Schedule::from::<FixedUpdate>());
        schedule.add_schedule(Schedule::from::<PostFixedUpdate>());
        schedule
    }
}
pub struct PreFixedUpate;
impl Phase for PreFixedUpate {}
pub struct FixedUpdate;
impl Phase for FixedUpdate {}
pub struct PostFixedUpdate;
impl Phase for PostFixedUpdate {}

pub struct PreUpdate;
impl Phase for PreUpdate {}
pub struct Update;
impl Phase for Update {}
pub struct PostUpdate;
impl Phase for PostUpdate {}

pub struct PreRender;
impl Phase for PreRender {}
pub struct Render;
impl Phase for Render {}
pub struct PostRender;
impl Phase for PostRender {}

pub struct Execute;
impl Phase for Execute {
    fn schedule() -> Schedule {
        let mut schedule = Schedule::from::<Execute>();
        schedule.add_schedule(Schedule::from::<Fixed>());
        schedule.add_schedule(Schedule::from::<PreUpdate>());
        schedule.add_schedule(Schedule::from::<Update>());
        schedule.add_schedule(Schedule::from::<PostUpdate>());
        schedule.add_schedule(Schedule::from::<PreRender>());
        schedule.add_schedule(Schedule::from::<Render>());
        schedule.add_schedule(Schedule::from::<PostRender>());
        schedule
    }
}

pub struct Shutdown;
impl Phase for Shutdown {}
