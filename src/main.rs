use ecs::{
    archetype::Archetypes,
    core::{allocator::Allocator, ComponentId},
    event::Event,
    system::schedule::Phase,
};

use crate::ecs::{
    core::Entities, event::Events, storage::table::ComponentSet, system::observer::Observers,
    world::World, Ecs,
};

pub mod ecs;

pub struct TestComponentA;
impl ecs::core::Component for TestComponentA {}

pub struct TestComponentB;
impl ecs::core::Component for TestComponentB {}

pub struct TestComponentC;
impl ecs::core::Component for TestComponentC {}

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
impl Phase for Update {
    fn name() -> &'static str {
        todo!()
    }

    fn run(&mut self, world: &World, schdeules: &ecs::system::schedule::Schedules) {
        todo!()
    }
}

fn main() {
    let mut world = World::new();
    world
        .register::<TestComponentA>()
        .register::<TestComponentB>()
        .register::<TestComponentC>();

    let entity1 = world.spawn(None);
    let entity2 = world.spawn(Some(entity1));

    world.add_component(&entity1, TestComponentA);
    world.add_component(&entity1, TestComponentB);
    world.add_component(&entity1, TestComponentC);

    world.add_component(&entity2, TestComponentA);
    world.add_component(&entity2, TestComponentB);
    world.remove_component(&entity2, &B);

    // world.query(&[A, B]).iter().for_each(|archetype| {
    //     let archetype = world.archetypes().get(archetype).unwrap();
    //     println!("Entities: {:?}", archetype.entities());
    // });

    {
        let parent1 = world.entities().parent(&entity1);
        let children = world.entities().children(&entity1);
        println!("Parent1: {:?}", parent1);
        println!("Children: {:?}", children);

        let parent2 = world.entities().parent(&entity2);
        let children = world.entities().children(&entity2);
        println!("Parent2: {:?}", parent2);
        println!("Children: {:?}", children);
    }

    let despawned = world.despawn(&entity1);

    for entity in despawned.keys() {
        println!("Despawned: {:?}", entity);
    }
}
