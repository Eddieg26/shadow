use ecs::{
    archetype::Archetypes,
    core::{allocator::Allocator, ComponentId},
    event::Event,
    system::schedule::Phase,
};

use crate::ecs::{core::Entities, event::Events, system::observer::Observers, world::World, Ecs};

pub mod ecs;

const A: ComponentId = ComponentId::new(0);
const B: ComponentId = ComponentId::new(1);
const C: ComponentId = ComponentId::new(2);

pub struct TestEvent;
impl Event for TestEvent {
    type Output = ();

    fn invoke(&mut self, world: &mut ecs::world::World) -> Self::Output {
        ()
    }
}

pub struct Update;
impl Phase for Update {}

fn main() {
    let mut allocator = Allocator::new();
    let mut archetypes = Archetypes::new();

    let entity1 = allocator.allocate().into();
    let entity2 = allocator.allocate().into();

    archetypes.add_entity(&entity1);
    archetypes.add_entity(&entity2);

    // archetypes.add_component(&entity1, B);
    // archetypes.add_component(&entity1, C);

    // archetypes.add_component(&entity2, A);
    // archetypes.add_component(&entity2, B);
    // archetypes.remove_component(&entity2, A);

    println!("Query 1");
    let query = archetypes.query(&[A]);
    println!("{:?}", query);
    for id in query {
        let archetype = archetypes.get(&id).unwrap();
        println!(
            "Components: {:?}",
            archetype.components().iter().collect::<Vec<_>>()
        );
    }

    let mut test_observers = Observers::<TestEvent>::new();
    test_observers.add(|events: &[()], world: &World| {
        for event in events {
            println!("TestEvent: {:?}", event);
        }
    });

    let mut ecs = Ecs::new();
    ecs.add_system::<Update, _>(|events: &Events| {
        events.add(TestEvent);
    })
    .register_event::<TestEvent>()
    .add_observers::<TestEvent>(test_observers)
    .add_observer::<TestEvent, _>(|events: &[()], world: &World| {
        for event in events {
            println!("TestEvent: {:?}", event);
        }
    });
}
