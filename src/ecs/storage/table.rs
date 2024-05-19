use super::dense::{DenseMap, DenseSet};
use crate::ecs::core::{
    internal::{blob::Blob, ptr::Ptr},
    ComponentId, Entity,
};
use std::collections::HashMap;

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

    pub fn from_column(column: &Column) -> Self {
        Column {
            data: column.data.copy(1),
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

    pub fn remove<T>(&mut self, index: usize) -> Option<T> {
        self.data.remove(index)
    }

    pub fn data(&self) -> &Blob {
        &self.data
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
    columns: DenseMap<ComponentId, Column>,
}

impl Row {
    pub fn new() -> Self {
        Row {
            columns: DenseMap::new(),
        }
    }

    pub fn with_columns(columns: DenseMap<ComponentId, Column>) -> Self {
        Row { columns }
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn add_column(&mut self, component: ComponentId, column: Column) {
        self.columns.insert(component, column);
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

    pub fn remove_at(&mut self, index: usize) -> Option<(ComponentId, Column)> {
        self.columns.remove_at(index)
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

    pub fn take(&mut self, columns: &DenseSet<ComponentId>) -> Row {
        let mut row = Row::new();
        for component in columns.iter() {
            if let Some(column) = self.columns.remove(component) {
                row.columns.insert(*component, column);
            } else {
                panic!("Component not found: {}", component);
            }
        }
        row
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
        for (component, column) in self.columns.iter() {
            let cell = TableCell { column, index };
            cells.insert(*component, cell);
        }

        Some(SelectedRow { cells })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ComponentId, &Column)> {
        self.columns.iter()
    }

    pub fn keys(&self) -> &[ComponentId] {
        self.columns.keys()
    }

    pub fn values(&self) -> &[Column] {
        self.columns.values()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn sort(&mut self) {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ComponentId, Column)> + '_ {
        self.columns.drain()
    }

    pub fn clear(&mut self) {
        for (_, column) in self.columns.iter_mut() {
            column.clear();
        }
    }
}

type Columns = Row;

pub struct TableBuilder {
    columns: DenseMap<ComponentId, Column>,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder {
            columns: DenseMap::new(),
        }
    }

    pub fn with_column(mut self, component: ComponentId, column: Column) -> Self {
        self.columns.insert(component, column);
        self
    }

    pub fn build(mut self) -> Table {
        self.columns.sort(|a, b| a.0.cmp(&b.0));
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
    pub fn new() -> TableBuilder {
        TableBuilder::new()
    }

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

    pub fn contains(&self, entity: &Entity) -> bool {
        self.rows.contains(entity)
    }

    pub fn entities(&self) -> &[Entity] {
        self.rows.values()
    }

    pub fn component_ids(&self) -> &[ComponentId] {
        self.columns.keys()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.columns.clear();
    }
}
