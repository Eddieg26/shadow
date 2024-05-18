use ecs::{
    archetype::Archetypes,
    core::{allocator::Allocator, ComponentId},
};

pub mod ecs;

const A: ComponentId = ComponentId::new(0);
const B: ComponentId = ComponentId::new(1);
const C: ComponentId = ComponentId::new(2);

fn main() {
    let mut allocator = Allocator::new();
    let mut archetypes = Archetypes::new();

    let entity1 = allocator.allocate().into();
    let entity2 = allocator.allocate().into();

    archetypes.add_entity(&entity1);
    archetypes.add_entity(&entity2);

    archetypes.add_component(&entity1, A);
    // archetypes.add_component(&entity1, B);
    // archetypes.add_component(&entity1, C);
    archetypes.add_components(&entity1, &[B, C]);

    // archetypes.add_component(&entity2, A);
    // archetypes.add_component(&entity2, B);
    // archetypes.remove_component(&entity2, A);

    println!("Query 1");
    let query = archetypes.query(&[A]);
    println!("{:?}", query);
    for id in query {
        let archetype = archetypes.get(&id).unwrap();
        println!("Components: {:?}", archetype.components().iter().collect::<Vec<_>>());
    }
}
