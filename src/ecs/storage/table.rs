use crate::ecs::{
    core::{internal::blob::Blob, Component, ComponentId, Entity},
    storage::dense::{DenseMap, DenseSet, ImmutableDenseMap},
};

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<C>() -> Self {
        Column {
            data: Blob::new::<C>(),
        }
    }

    pub fn from_column(column: &Column) -> Self {
        Column {
            data: column.data.copy(1),
        }
    }

    pub fn from_blob(data: Blob) -> Self {
        Column { data }
    }

    pub fn cell(&self, index: usize) -> TableCell {
        TableCell {
            column: self,
            index,
        }
    }

    pub fn get<C>(&self, index: usize) -> Option<&C> {
        self.data.get(index)
    }

    pub fn get_mut<C>(&self, index: usize) -> Option<&mut C> {
        self.data.get_mut(index)
    }

    pub fn insert<C>(&mut self, value: C) {
        self.data.push(value);
    }

    pub fn push(&mut self, mut value: Blob) {
        self.data.append(&mut value);
    }

    pub fn remove<C>(&mut self, index: usize) -> Option<C> {
        self.data.remove(index)
    }

    pub fn swap_remove(&mut self, index: usize) -> Blob {
        self.data.swap_remove(index)
    }

    pub fn data(&self) -> &Blob {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

pub struct TableCell<'a> {
    column: &'a Column,
    index: usize,
}

impl<'a> TableCell<'a> {
    pub fn cast<C>(&self) -> Option<&C> {
        self.column.get(self.index)
    }

    pub fn cast_mut<C>(&self) -> Option<&mut C> {
        self.column.get_mut(self.index)
    }
}

pub struct RowLayout {
    columns: DenseMap<ComponentId, Column>,
}

impl RowLayout {
    pub fn new() -> Self {
        RowLayout {
            columns: DenseMap::new(),
        }
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn add_column(&mut self, component: ComponentId, column: Column) {
        self.columns.insert(component, column);
    }

    pub fn build(mut self) -> Row {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
        Row {
            columns: self.columns.to_immutable(),
        }
    }
}

pub struct Row {
    columns: ImmutableDenseMap<ComponentId, Column>,
}

impl Row {
    pub fn new() -> RowLayout {
        RowLayout::new()
    }

    pub fn column(&self, component: ComponentId) -> Option<&Column> {
        self.columns.get(&component)
    }

    pub fn column_mut(&mut self, component: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(&component)
    }

    pub fn select(&self, component: ComponentId) -> Option<TableCell> {
        self.columns.get(&component).map(|column| column.cell(0))
    }

    pub fn set(&mut self, component: ComponentId, column: Column) {
        self.columns.get_mut(&component).map(|c| *c = column);
    }

    pub fn remove(&mut self, components: impl Into<DenseSet<ComponentId>>) -> Row {
        let components = components.into();
        let mut layout = RowLayout::new();
        for component in components.iter() {
            if let Some(column) = self.columns.get_mut(&component) {
                let data = column.swap_remove(0);
                layout.add_column(*component, Column::from_blob(data));
            }
        }

        layout.build()
    }
}

pub struct SelectedRow<'a> {
    cells: ImmutableDenseMap<ComponentId, TableCell<'a>>,
}

impl<'a> SelectedRow<'a> {
    pub fn get<C>(&self, component: ComponentId) -> Option<&C> {
        self.cells.get(&component).and_then(|cell| cell.cast())
    }

    pub fn get_mut<C>(&self, component: ComponentId) -> Option<&mut C> {
        self.cells.get(&component).and_then(|cell| cell.cast_mut())
    }
}

pub struct TableLayout {
    columns: DenseMap<ComponentId, Column>,
}

impl TableLayout {
    pub fn new() -> Self {
        TableLayout {
            columns: DenseMap::new(),
        }
    }

    pub fn with_components(mut self, components: ComponentSet) -> Self {
        self.columns = components.into();
        self
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn add_column(&mut self, component: ComponentId, column: Column) {
        self.columns.insert(component, column);
    }

    pub fn build(mut self) -> Table {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
        Table {
            columns: self.columns.to_immutable(),
            entities: DenseSet::new(),
        }
    }
}

pub struct Table {
    columns: ImmutableDenseMap<ComponentId, Column>,
    entities: DenseSet<Entity>,
}

impl Table {
    pub fn new() -> TableLayout {
        TableLayout::new()
    }

    pub fn components(&self) -> &[ComponentId] {
        self.columns.keys()
    }

    pub fn entities(&self) -> &[Entity] {
        self.entities.values()
    }

    pub fn column(&self, component: ComponentId) -> Option<&Column> {
        self.columns.get(&component)
    }

    fn column_mut(&mut self, component: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(&component)
    }

    pub fn select(&self, entity: &Entity) -> Option<SelectedRow> {
        self.entities.index(entity).map(|index| {
            let mut cells = DenseMap::new();
            for (component, column) in self.columns.iter() {
                let cell = column.cell(index);
                cells.insert(*component, cell);
            }

            SelectedRow {
                cells: cells.to_immutable(),
            }
        })
    }

    pub fn insert(&mut self, entity: &Entity, mut row: Row) {
        self.entities.insert(*entity);
        for (component, entity_column) in row.columns.iter_mut() {
            let column = self.column_mut(*component).expect("Component not found");
            column.push(entity_column.swap_remove(0));
        }
    }

    pub fn remove(&mut self, entity: &Entity) -> Option<ComponentSet> {
        self.entities.index(entity).map(|index| {
            self.entities.swap_remove(entity);
            let mut set = ComponentSet::new();
            for (id, column) in self.columns.iter_mut() {
                let data = column.swap_remove(index);
                set.add_column(*id, Column::from_blob(data));
            }

            set
        })
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.entities.contains(entity)
    }
}

pub struct ComponentSet {
    components: DenseMap<ComponentId, Column>,
}

impl ComponentSet {
    pub fn new() -> Self {
        ComponentSet {
            components: DenseMap::new(),
        }
    }

    pub fn ids(&self) -> &[ComponentId] {
        self.components.keys()
    }

    pub fn add_column(&mut self, component: ComponentId, column: Column) -> Option<Column> {
        self.components.insert(component, column)
    }

    pub fn with<C: Component>(mut self, id: ComponentId, component: C) -> Self {
        self.insert(id, component);
        self
    }

    pub fn insert<C: Component>(&mut self, id: ComponentId, component: C) -> Option<Column> {
        let mut column = Column::new::<C>();
        column.insert(component);
        self.components.insert(id, column)
    }

    pub fn get(&self, id: &ComponentId) -> Option<&Column> {
        self.components.get(id)
    }

    pub fn get_mut(&mut self, id: &ComponentId) -> Option<&mut Column> {
        self.components.get_mut(id)
    }

    pub fn components(&self) -> &DenseMap<ComponentId, Column> {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut DenseMap<ComponentId, Column> {
        &mut self.components
    }

    pub fn remove_at(&mut self, index: usize) -> Option<(ComponentId, Column)> {
        self.components.remove_at(index)
    }

    pub fn remove(&mut self, id: &ComponentId) -> Option<Column> {
        self.components.remove(id)
    }

    pub fn remove_component<C: Component>(&mut self, id: &ComponentId) -> Option<C> {
        self.components
            .get_mut(id)
            .and_then(|column| column.remove(0))
    }

    pub fn sort(&mut self) {
        self.components.sort(|a, b| a.0.cmp(&b.0));
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ComponentId, Column)> + '_ {
        self.components.drain()
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.len() == 0
    }

    pub fn has(&self, id: &ComponentId) -> bool {
        self.components.contains(id)
    }

    pub fn layout(&self) -> TableLayout {
        let mut layout = TableLayout::new();
        for (id, column) in self.components.iter() {
            layout.add_column(*id, Column::from_column(column));
        }

        layout
    }
}

impl Into<Row> for ComponentSet {
    fn into(mut self) -> Row {
        let mut layout = RowLayout::new();
        for (id, column) in self.components.drain() {
            layout.add_column(id, column);
        }

        layout.build()
    }
}

impl Into<DenseMap<ComponentId, Column>> for ComponentSet {
    fn into(self) -> DenseMap<ComponentId, Column> {
        self.components
    }
}
