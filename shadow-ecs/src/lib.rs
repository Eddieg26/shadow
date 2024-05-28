use crate::ecs::{event::Events, world::World};
use ecs::{
    core::{Entity, Resource},
    system::systems::{RunMode, Systems},
    world::{
        events::Spawn,
        query::{FilterQuery, Not, Query, With},
    },
};

pub mod ecs;

pub struct TestComponentA(u32);
impl ecs::core::Component for TestComponentA {}

pub struct TestComponentB;
impl ecs::core::Component for TestComponentB {}

pub struct TestComponentC;
impl ecs::core::Component for TestComponentC {}

pub struct TestResource;
impl Resource for TestResource {}

fn test_a(events: &Events) {
    println!("Entity Spawned");
    events.add(Spawn::new().with(TestComponentA(50)).with(TestComponentB));
}

fn test_b(test: &TestResource) {
    println!("TEST SYSTEM B");
}

fn test_c(test: &mut TestResource) {
    println!("TEST SYSTEM C");
}

fn observe_create(events: &[Entity]) {
    for entity in events {
        println!("Entity created: {:?}", entity);
    }
}

fn q(query: Query<&TestComponentA, Not<TestComponentB>>) {
    for entity in query {
        println!("Component: {}", entity.0)
    }
}

// fn main() {
//     let mut world = World::new();
//     let mut systems = Systems::new(RunMode::Sequential);

//     world
//         .register::<TestComponentA>()
//         .register::<TestComponentB>()
//         .register::<TestComponentC>()
//         .add_resource(TestResource)
//         .observe::<Spawn, _>(observe_create);

//     systems.add_system(test_a);
//     systems.add_system(test_b);
//     systems.add_system(test_c);
//     systems.build();
//     systems.run(&world);
//     world.flush();

//     let mut systems = Systems::new(RunMode::Sequential);
//     systems.add_system(q);
//     systems.build();
//     systems.run(&world);
// }
