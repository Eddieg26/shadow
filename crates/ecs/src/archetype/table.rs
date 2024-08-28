use crate::core::{
    Column, ColumnCell, ColumnKey, Component, ComponentId, DenseMap, DenseSet, Entity, Row,
};

pub struct EntityRow {
    components: DenseMap<ComponentId, ColumnCell>,
}

impl EntityRow {
    pub fn new() -> Self {
        EntityRow {
            components: DenseMap::new(),
        }
    }

    pub fn components(&self) -> &[ComponentId] {
        self.components.keys()
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.components
            .get(&ComponentId::new::<C>())
            .and_then(|cell| Some(cell.value::<C>()))
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.components
            .get_mut(&ComponentId::new::<C>())
            .and_then(|cell| Some(cell.value_mut::<C>()))
    }

    pub fn add_component<C: Component>(&mut self, component: C) -> Option<ColumnCell> {
        self.components
            .insert(ComponentId::new::<C>(), ColumnCell::from(component))
    }

    pub fn remove_component<C: Component>(&mut self) -> Option<C> {
        self.components
            .remove(&ComponentId::new::<C>())
            .and_then(|cell| Some(cell.take()))
    }

    pub fn add_cell(&mut self, id: ComponentId, cell: ColumnCell) -> Option<ColumnCell> {
        self.components.insert(id, cell)
    }

    pub fn remove_cell(&mut self, id: &ComponentId) -> Option<ColumnCell> {
        self.components.remove(id)
    }

    pub fn contains<C: Component>(&self) -> bool {
        self.components.contains(&ComponentId::new::<C>())
    }

    pub fn contains_id(&self, id: &ComponentId) -> bool {
        self.components.contains(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ComponentId, &ColumnCell)> {
        self.components.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ComponentId, &mut ColumnCell)> {
        self.components.iter_mut()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ComponentId, ColumnCell)> + '_ {
        self.components.drain()
    }

    pub fn sort(&mut self) {
        self.components.sort(|a, b| a.cmp(b));
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub fn into_table(mut self, entity: Entity) -> EntityTable {
        let mut builder = TableBuilder::new();
        for (id, cell) in self.components.drain() {
            builder.add_column(id, Column::from(cell));
        }
        let mut table = builder.build();
        table.rows.insert(entity);

        table
    }

    pub fn table_builder(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        for (id, cell) in self.components.iter() {
            builder.add_column(*id, Column::copy_cell(cell));
        }

        builder
    }
}

impl From<Row> for EntityRow {
    fn from(mut row: Row) -> Self {
        let mut components = DenseMap::new();
        row.drain().for_each(|(id, cell)| {
            components.insert(ComponentId::raw(*id), cell);
        });

        EntityRow { components }
    }
}

impl Into<Row> for EntityRow {
    fn into(mut self) -> Row {
        let mut row = Row::new();
        self.components.drain().for_each(|(id, cell)| {
            let key = ColumnKey::raw(*id);
            row.add_cell(key, cell);
        });

        row
    }
}

pub struct SelectedRow<'a> {
    index: usize,
    row: DenseMap<ComponentId, &'a Column>,
}

impl<'a> SelectedRow<'a> {
    pub fn new(index: usize, row: DenseMap<ComponentId, &'a Column>) -> Self {
        SelectedRow { index, row }
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.row
            .get(&ComponentId::new::<C>())
            .and_then(|column| column.get(self.index))
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.row
            .get_mut(&ComponentId::new::<C>())
            .and_then(|column| column.get_mut(self.index))
    }

    pub fn contains<C: Component>(&self) -> bool {
        self.row.contains(&ComponentId::new::<C>())
    }

    pub fn contains_id(&self, id: &ComponentId) -> bool {
        self.row.contains(id)
    }
}

pub struct TableBuilder {
    components: DenseMap<ComponentId, Column>,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder {
            components: DenseMap::new(),
        }
    }

    pub fn components(&self) -> &[ComponentId] {
        self.components.keys()
    }

    pub fn add_component<C: Component>(&mut self) {
        self.components
            .insert(ComponentId::new::<C>(), Column::new::<C>());
    }

    pub fn remove_component<C: Component>(&mut self) {
        self.components.remove(&ComponentId::new::<C>());
    }

    pub fn add_column(&mut self, id: ComponentId, column: Column) {
        self.components.insert(id, column);
    }

    pub fn remove_column(&mut self, id: &ComponentId) {
        self.components.remove(id);
    }

    pub fn build(mut self) -> EntityTable {
        self.components.sort(|a, b| a.cmp(b));
        EntityTable {
            rows: DenseSet::new(),
            components: self.components,
        }
    }
}

pub struct EntityTable {
    rows: DenseSet<Entity>,
    components: DenseMap<ComponentId, Column>,
}

impl EntityTable {
    pub fn builder() -> TableBuilder {
        TableBuilder::new()
    }

    pub fn entities(&self) -> &[Entity] {
        self.rows.keys()
    }

    pub fn components(&self) -> &[ComponentId] {
        self.components.keys()
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.rows.contains(entity)
    }

    pub fn has_component(&self, id: &ComponentId) -> bool {
        self.components.contains(id)
    }

    pub fn get_component<C: Component>(&self, entity: &Entity) -> Option<&C> {
        let column = self.components.get(&ComponentId::new::<C>())?;
        let index = self.rows.index_of(entity)?;
        column.get(index)
    }

    pub fn get_component_mut<C: Component>(&self, entity: &Entity) -> Option<&mut C> {
        let column = self.components.get(&ComponentId::new::<C>())?;
        let index = self.rows.index_of(entity)?;
        column.get_mut(index)
    }

    pub fn add_entity(&mut self, entity: Entity, mut row: EntityRow) {
        self.rows.insert(entity);
        for (id, cell) in row.drain() {
            let column = match self.components.get_mut(&id) {
                Some(column) => column,
                None => continue,
            };

            column.push_cell(cell);
        }
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> Option<EntityRow> {
        let index = self.rows.remove(entity)?;
        let mut row = EntityRow::new();
        for (id, column) in self.components.iter_mut() {
            let cell = column.remove_cell(index);
            row.add_cell(*id, cell);
        }

        Some(row)
    }
}
