use super::dense::{DenseMap, DenseSet};
use crate::ecs::core::{
    internal::{blob::Blob, ptr::Ptr},
    ComponentId, Entity,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<T>() -> Self {
        Column {
            data: Blob::new::<T>(),
        }
    }

    pub fn with_capacity<T>(capacity: usize) -> Self {
        Column {
            data: Blob::with_capacity::<T>(capacity),
        }
    }

    pub fn from_blob(blob: Blob) -> Self {
        Column { data: blob }
    }

    pub fn push<T>(&mut self, value: T) {
        self.data.push(value);
    }

    pub fn append(&mut self, mut column: Column) {
        self.data.append(&mut column.data);
    }

    pub fn swap_remove(&mut self, index: usize) -> Blob {
        self.data.swap_remove(index)
    }

    pub fn ptr(&self, index: usize) -> Option<Ptr> {
        if index < self.data.len() {
            unsafe { Some(self.data.ptr().add(index)) }
        } else {
            None
        }
    }

    pub fn get<T>(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    pub fn get_mut<T>(&self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
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
    pub fn get<T>(&self) -> Option<&T> {
        self.column.get(self.index)
    }

    pub fn get_mut<T>(&self) -> Option<&mut T> {
        self.column.get_mut(self.index)
    }
}

pub struct SelectedRow<'a> {
    cells: HashMap<ComponentId, TableCell<'a>>,
}

impl<'a> SelectedRow<'a> {
    pub fn get<T>(&self, component: ComponentId) -> Option<&T> {
        self.cells.get(&component).and_then(|cell| cell.get())
    }

    pub fn get_mut<T>(&self, component: ComponentId) -> Option<&mut T> {
        self.cells.get(&component).and_then(|cell| cell.get_mut())
    }
}

pub struct Row {
    columns: HashMap<ComponentId, Column>,
}

impl Row {
    pub fn new() -> Self {
        Row {
            columns: HashMap::new(),
        }
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn remove_column(&mut self, component: ComponentId) -> Option<Column> {
        self.columns.remove(&component)
    }

    pub fn append(&mut self, mut row: Row) {
        for (component, column) in row.columns.drain() {
            if let Some(found) = self.columns.get_mut(&component) {
                found.append(column);
            } else {
                self.columns.insert(component, column);
            }
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Row {
        let mut row = Row::new();
        for (id, column) in self.columns.iter_mut() {
            let column = column.swap_remove(index);
            row.columns.insert(*id, Column::from_blob(column));
        }
        row
    }

    pub fn retain(&mut self, components: &[ComponentId]) {
        self.columns
            .retain(|component, _| components.contains(component));
    }

    pub fn get<T>(&self, component: ComponentId) -> Option<&T> {
        self.columns
            .get(&component)
            .and_then(|column| column.get(0))
    }

    pub fn get_mut<T>(&self, component: ComponentId) -> Option<&mut T> {
        self.columns
            .get(&component)
            .and_then(|column| column.get_mut(0))
    }

    pub fn select(&self, index: usize) -> Option<SelectedRow> {
        let mut cells = HashMap::new();
        for (component, column) in &self.columns {
            let cell = TableCell { column, index };
            cells.insert(*component, cell);
        }

        Some(SelectedRow { cells })
    }

    pub fn clear(&mut self) {
        self.columns.clear();
    }
}

type Columns = Row;

pub struct TableBuilder {
    columns: HashMap<ComponentId, Column>,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder {
            columns: HashMap::new(),
        }
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn build(self) -> Table {
        Table {
            columns: Row {
                columns: self.columns,
            },
            rows: DenseSet::new(),
        }
    }
}

pub struct Table {
    columns: Columns,
    rows: DenseSet<Entity>,
}

impl Table {
    pub fn select(&self, entity: &Entity) -> Option<SelectedRow> {
        self.rows
            .index(entity)
            .and_then(|index| self.columns.select(index))
    }

    pub fn insert(&mut self, entity: Entity, row: Row) {
        self.rows.insert(entity);
        self.columns.append(row);
    }

    pub fn remove(&mut self, entity: &Entity) -> Option<Row> {
        if let Some(index) = self.rows.index(entity) {
            self.rows.remove_at(index);
            Some(self.columns.swap_remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.columns.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct TableId(u64);

impl TableId {
    pub fn new(ids: &[ComponentId]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ids.hash(&mut hasher);
        TableId(hasher.finish())
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<u64> for TableId {
    fn from(id: u64) -> Self {
        TableId(id)
    }
}

pub struct Tables {
    tables: DenseMap<TableId, Table>,
}

impl Tables {
    pub fn new() -> Self {
        Tables {
            tables: DenseMap::new(),
        }
    }

    pub fn insert(&mut self, id: TableId, table: Table) {
        self.tables.insert(id, table);
    }

    pub fn remove(&mut self, id: &TableId) -> Option<Table> {
        self.tables.remove(id)
    }

    pub fn get(&self, id: &TableId) -> Option<&Table> {
        self.tables.get(id)
    }

    pub fn get_mut(&mut self, id: &TableId) -> Option<&mut Table> {
        self.tables.get_mut(id)
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }

    pub fn slice(&self) -> &[Table] {
        self.tables.values()
    }

    pub fn select<'a>(&'a self, tables: &'a [TableId]) -> impl Iterator<Item = &Table> {
        self.tables.iter().filter_map(|(id, table)| {
            if tables.contains(id) {
                Some(table)
            } else {
                None
            }
        })
    }
}
