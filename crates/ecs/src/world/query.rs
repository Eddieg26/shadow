use crate::{
    archetype::{Archetype, ArchetypeId},
    core::{Component, ComponentId, Entity},
    system::{
        access::{Access, WorldAccess, WorldAccessType},
        SystemArg,
    },
};
use std::collections::HashSet;

use super::World;

pub trait BaseQuery {
    type Item<'a>: Send + Sync;

    fn init(_: &World, _: &mut QueryState) {}
    fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_>;
    fn access() -> Vec<WorldAccess>;
}

impl<C: Component> BaseQuery for &C {
    type Item<'a> = &'a C;

    fn init(_: &World, state: &mut QueryState) {
        state.add_component(ComponentId::new::<C>());
    }

    fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_> {
        archetype.component::<C>(&entity).unwrap()
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::new(
            WorldAccessType::Component(ComponentId::new::<C>()),
            Access::Read,
        )]
    }
}

impl<C: Component> BaseQuery for &mut C {
    type Item<'a> = &'a mut C;

    fn init(_: &World, state: &mut QueryState) {
        state.add_component(ComponentId::new::<C>());
    }

    fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_> {
        archetype.component_mut::<C>(&entity).unwrap()
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::new(
            WorldAccessType::Component(ComponentId::new::<C>()),
            Access::Write,
        )]
    }
}

impl<C: Component> BaseQuery for Option<&C> {
    type Item<'a> = Option<&'a C>;

    fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_> {
        archetype.component::<C>(&entity)
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::new(
            WorldAccessType::Component(ComponentId::new::<C>()),
            Access::Read,
        )]
    }
}

impl<C: Component> BaseQuery for Option<&mut C> {
    type Item<'a> = Option<&'a mut C>;

    fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_> {
        archetype.component_mut::<C>(&entity)
    }

    fn access() -> Vec<WorldAccess> {
        vec![WorldAccess::new(
            WorldAccessType::Component(ComponentId::new::<C>()),
            Access::Write,
        )]
    }
}

impl BaseQuery for Entity {
    type Item<'a> = Entity;

    fn fetch(_: &Archetype, entity: Entity) -> Self::Item<'_> {
        entity
    }

    fn access() -> Vec<WorldAccess> {
        let ty = WorldAccessType::None;
        vec![WorldAccess::new(ty, Access::Read)]
    }
}

pub trait FilterQuery {
    fn init(world: &World, state: &mut QueryState);
}

pub struct With<C: Component> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: Component> FilterQuery for With<C> {
    fn init(_: &World, state: &mut QueryState) {
        state.add_component(ComponentId::new::<C>());
    }
}

pub struct Not<C: Component> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: Component> FilterQuery for Not<C> {
    fn init(_: &World, state: &mut QueryState) {
        state.exclude(ComponentId::new::<C>());
    }
}

impl FilterQuery for () {
    fn init(_: &World, _: &mut QueryState) {}
}

pub struct Query<'a, Q: BaseQuery, F: FilterQuery = ()> {
    world: &'a World,
    archetypes: Vec<ArchetypeId>,
    row_index: usize,
    archetype_index: usize,
    archetype: Option<&'a Archetype>,
    _marker: std::marker::PhantomData<(Q, F)>,
}

impl<'a, Q: BaseQuery, F: FilterQuery> Query<'a, Q, F> {
    pub fn new(world: &'a World) -> Self {
        let mut state = QueryState::new();
        Q::init(world, &mut state);
        F::init(world, &mut state);

        let archetypes = world
            .archetypes()
            .query(state.components(), &state.excluded);
        let archetype = archetypes.get(0).and_then(|id| world.archetypes().get(id));

        Self {
            world,
            archetypes,
            archetype_index: 0,
            row_index: 0,
            archetype,
            _marker: std::marker::PhantomData,
        }
    }

    pub unsafe fn get<C: Component>(&self, entity: Entity) -> Option<&C> {
        let archetypes = self.world.archetypes();
        archetypes
            .entity_archetype(&entity)
            .and_then(|id| archetypes.get(&id))
            .and_then(|archetype| archetype.component::<C>(&entity))
    }

    pub unsafe fn get_mut<C: Component>(&self, entity: Entity) -> Option<&mut C> {
        let archetypes = self.world.archetypes();
        archetypes
            .entity_archetype(&entity)
            .and_then(|id| archetypes.get(&id))
            .and_then(|archetype| archetype.component_mut::<C>(&entity))
    }
}

#[derive(Clone)]
pub struct QueryState {
    components: Vec<ComponentId>,
    excluded: HashSet<ComponentId>,
}

impl QueryState {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            excluded: HashSet::new(),
        }
    }

    pub fn add_component(&mut self, component: ComponentId) {
        self.components.push(component);
    }

    pub fn exclude(&mut self, component: ComponentId) {
        self.excluded.insert(component);
    }

    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }

    pub fn excluded(&self) -> &HashSet<ComponentId> {
        &self.excluded
    }
}

impl<'a, Q: BaseQuery, F: FilterQuery> Iterator for Query<'a, Q, F> {
    type Item = Q::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.archetype_index >= self.archetypes.len() {
            return None;
        } else if self.row_index >= self.archetype.unwrap().entities().len() {
            self.archetype_index += 1;
            self.row_index = 0;
            self.archetype = self
                .archetypes
                .get(self.archetype_index)
                .and_then(|id| self.world.archetypes().get(id));

            return self.next();
        } else {
            let archetype = self.archetype.unwrap();
            let entity = archetype.entities()[self.row_index];
            self.row_index += 1;

            Some(Q::fetch(archetype, entity))
        }
    }
}

impl<Q: BaseQuery, F: FilterQuery> SystemArg for Query<'_, Q, F> {
    type Item<'a> = Query<'a, Q, F>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        Query::new(world)
    }

    fn access() -> Vec<WorldAccess> {
        Q::access()
    }
}

#[macro_export]
macro_rules! impl_base_query_for_tuples {
    ($(($($name:ident),+)),+) => {
        $(
            impl<$($name: BaseQuery),+> BaseQuery for ($($name,)+) {
                type Item<'a> = ($($name::Item<'a>,)+);

                fn init(world: &World, state: &mut QueryState) {
                    $(
                        $name::init(world, state);
                    )+
                }

                fn fetch(archetype: &Archetype, entity: Entity) -> Self::Item<'_> {
                    ($($name::fetch(archetype, entity),)+)
                }

                fn access() -> Vec<WorldAccess> {
                    let mut metas = Vec::new();
                    $(
                        metas.extend($name::access());
                    )+
                    metas
                }
            }
        )+
    };
}

#[macro_export]
macro_rules! impl_filter_query_for_tuple {
    ($($filter:ident),*) => {
        impl<$($filter: FilterQuery),*> FilterQuery for ($($filter,)*) {
            fn init(world: &World, state: &mut QueryState) {
                $(
                    $filter::init(world, state);
                )*
            }
        }
    };
}

impl_base_query_for_tuples!((A, B));
impl_base_query_for_tuples!((A, B, C));
impl_base_query_for_tuples!((A, B, C, D));
impl_base_query_for_tuples!((A, B, C, D, E));
impl_base_query_for_tuples!((A, B, C, D, E, F));
impl_base_query_for_tuples!((A, B, C, D, E, F, G));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P));
impl_base_query_for_tuples!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::table::EntityRow;
    use crate::core::Component;
    use crate::world::World;

    struct A;
    struct B;
    struct C;

    impl Component for A {}
    impl Component for B {}
    impl Component for C {}

    #[test]
    fn query() {
        let mut world = World::new();
        let entity = world.spawn();
        let mut components = EntityRow::new();
        components.add_component(A);
        components.add_component(B);
        components.add_component(C);
        world.add_components(&entity, components);

        let mut query = Query::<(&A, &B, &C)>::new(&world);
        match query.next() {
            Some((a, b, c)) => (a, b, c),
            None => panic!("Query returned None"),
        };
    }

    #[test]
    fn query_filter_with() {
        let mut world = World::new();
        let entity = world.spawn();
        let mut components = EntityRow::new();
        components.add_component(A);
        components.add_component(B);
        components.add_component(C);
        world.add_components(&entity, components);

        let mut query = Query::<(&A, &B), With<C>>::new(&world);
        match query.next() {
            Some((a, b)) => (a, b),
            None => panic!("Query returned None"),
        };
    }

    #[test]
    fn query_filter_not() {
        let mut world = World::new();
        let entity = world.spawn();
        let mut components = EntityRow::new();
        components.add_component(A);
        components.add_component(B);
        world.add_components(&entity, components);

        let mut query = Query::<(&A, &B), Not<C>>::new(&world);
        match query.next() {
            Some((a, b)) => (a, b),
            None => panic!("Query returned None"),
        };

        let mut query = Query::<(&A, &B, &C)>::new(&world);
        match query.next() {
            Some((..)) => panic!("Query returned Some"),
            None => (),
        };
    }
}
