use super::{
    access::{WorldAccess, WorldAccessType},
    ArgItem, SystemArg,
};
use crate::ecs::{
    core::internal::blob::Blob,
    event::{Event, EventOutputs, EventType},
    storage::dense::DenseMap,
    world::World,
};
use std::any::TypeId;

pub struct Observer<E: Event> {
    function: Box<dyn Fn(&[E::Output], &World) + Send + Sync + 'static>,
    reads: Vec<WorldAccessType>,
    writes: Vec<WorldAccessType>,
}

impl<E: Event> Observer<E> {
    pub fn new(
        function: impl Fn(&[E::Output], &World) + Send + Sync + 'static,
        reads: Vec<WorldAccessType>,
        writes: Vec<WorldAccessType>,
    ) -> Self {
        Self {
            function: Box::new(function),
            reads,
            writes,
        }
    }

    pub fn reads(&self) -> &[WorldAccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[WorldAccessType] {
        &self.writes
    }

    pub fn run(&self, output: &[E::Output], world: &World) {
        (self.function)(output, world);
    }
}

pub struct Observers<E: Event> {
    observers: Vec<Observer<E>>,
}

impl<E: Event> Observers<E> {
    pub fn new() -> Self {
        Self { observers: vec![] }
    }

    pub fn add<M>(&mut self, observer: impl IntoObserver<E, M>) {
        self.observers.push(observer.into_observer());
    }
}

pub struct ErasedObservers {
    ty: EventType,
    observers: Blob,
    observe: Box<dyn Fn(&Blob, &World) + Send + Sync + 'static>,
}

impl ErasedObservers {
    pub fn new<E: Event>() -> Self {
        Self {
            ty: TypeId::of::<E>(),
            observers: Blob::new::<Observer<E>>(),
            observe: Box::new(|blob, world| {
                let outputs = world.resource_mut::<EventOutputs<E>>().drain();
                for observer in blob.iter::<Observer<E>>() {
                    observer.run(&outputs, world);
                }
            }),
        }
    }

    pub fn add_observer<E: Event>(&mut self, observer: Observer<E>) {
        let ty = TypeId::of::<E>();
        if self.ty != ty {
            panic!("Event type mismatch!");
        }
        self.observers.push(observer);
    }

    pub fn add_observers<E: Event>(&mut self, observers: Observers<E>) {
        let ty = TypeId::of::<E>();
        if self.ty != ty {
            panic!("Event type mismatch!");
        }
        let mut observers = observers;
        for observer in observers.observers.drain(..) {
            self.observers.push(observer);
        }
    }

    pub fn observe(&self, world: &World) {
        (self.observe)(&self.observers, world);
    }
}

pub struct EventObservers {
    observers: DenseMap<EventType, ErasedObservers>,
}

impl EventObservers {
    pub fn new() -> Self {
        Self {
            observers: DenseMap::new(),
        }
    }

    pub fn add_observer<E: Event, M>(&mut self, observer: impl IntoObserver<E, M>) {
        let ty = TypeId::of::<E>();
        if let Some(erased) = self.observers.get_mut(&ty) {
            erased.add_observer(observer.into_observer());
        } else {
            let mut erased = ErasedObservers::new::<E>();
            erased.add_observer(observer.into_observer());
            self.observers.insert(ty, erased);
        }
    }

    pub fn add_observers<E: Event>(&mut self, observers: Observers<E>) {
        let ty = TypeId::of::<E>();
        if let Some(erased) = self.observers.get_mut(&ty) {
            erased.add_observers(observers);
        } else {
            let mut erased = ErasedObservers::new::<E>();
            erased.add_observers(observers);
            self.observers.insert(ty, erased);
        }
    }

    pub fn run(&self, world: &World) {
        for invocation in world.events().invocations() {
            if let Some(observers) = self.observers.get(&invocation.event()) {
                observers.observe(world);
                let meta = world.events().meta_dynamic(&invocation.event());
                meta.clear(world);
            }
        }
    }

    pub fn run_type<E: Event>(&self, world: &World) {
        if let Some(invocation) = world.events().invocation_type::<E>() {
            if let Some(observers) = self.observers.get(&invocation.event()) {
                observers.observe(world);
                let meta = world.events().meta_dynamic(&invocation.event());
                meta.clear(world);
            }
        }
    }
}

pub trait IntoObserver<E: Event, M> {
    fn into_observer(self) -> Observer<E>;
}

impl<E: Event, F> IntoObserver<E, ()> for F
where
    F: Fn(&[E::Output]) + Send + Sync + 'static,
{
    fn into_observer(self) -> Observer<E> {
        Observer::new(
            move |outputs: &[E::Output], _: &World| {
                (self)(outputs);
            },
            vec![],
            vec![],
        )
    }
}

impl<E: Event> IntoObserver<E, ()> for Observer<E> {
    fn into_observer(self) -> Observer<E> {
        self
    }
}

macro_rules! impl_into_observer {
    ($($arg:ident),*) => {
        impl<Ev: Event, F, $($arg: SystemArg),*> IntoObserver<Ev, (F, $($arg),*)> for F
        where
            for<'a> F: Fn(&[Ev::Output], $($arg),*) + Fn(&[Ev::Output], $(ArgItem<'a, $arg>),*) + Send + Sync +'static,
        {
            fn into_observer(self) -> Observer<Ev> {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::access());)*

                WorldAccess::pick(&mut reads, &mut writes, &metas);

                let system = Observer::<Ev>::new(move |outputs: &[Ev::Output], world: &World| {
                    (self)(outputs, $($arg::get(world)),*);
                }, reads, writes);

                system
            }
        }
    };
}

impl_into_observer!(A);
impl_into_observer!(A, B);
impl_into_observer!(A, B, C);
impl_into_observer!(A, B, C, D);
impl_into_observer!(A, B, C, D, E);
impl_into_observer!(A, B, C, D, E, F2);
