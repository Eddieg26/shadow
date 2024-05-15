use crate::ecs::core::GenId;

use super::{
    blob::Blob,
    ptr::Ptr,
    sparse::{ImmutableSparseSet, SparseMap, SparseSet},
};
use std::hash::{Hash, Hasher};

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<T>() -> Self {
        Self {
            data: Blob::new::<T>(),
        }
    }

    pub fn copy(&self, capacity: usize) -> Self {
        Self {
            data: self.data.copy(capacity),
        }
    }

    pub fn with_capacity<T>(capacity: usize) -> Self {
        Self {
            data: Blob::with_capacity::<T>(capacity),
        }
    }

    pub fn from_blob(blob: Blob) -> Self {
        Self { data: blob }
    }

    pub fn push<T>(&mut self, value: T) {
        self.data.push(value);
    }

    fn push_blob(&mut self, mut blob: Blob) {
        self.data.append(&mut blob);
    }

    pub fn swap_remove(&mut self, index: usize) -> Blob {
        self.data.swap_remove(index)
    }

    pub fn offset(&self, index: usize) -> Option<Ptr> {
        if index < self.data.len() {
            Some(self.data.ptr().add(index))
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

    pub fn ptr(&self) -> Ptr {
        self.data.ptr()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row(usize);

impl Row {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

impl std::ops::Deref for Row {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Row {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct TableBuilder<I: Into<GenId> + Clone> {
    columns: SparseSet<Column>,
    capacity: usize,
    _marker: std::marker::PhantomData<I>,
}

impl<I: Into<GenId> + Clone> TableBuilder<I> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            columns: SparseSet::with_capacity(capacity),
            capacity,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn add_column(mut self, index: usize, column: Column) -> Self {
        self.columns.insert(index, column);

        self
    }

    pub fn build(self) -> Table<I> {
        Table {
            id: TableId::new(&self.columns.indices().collect::<Vec<_>>()),
            columns: self.columns.into_immutable(),
            rows: Vec::with_capacity(self.capacity),
            sparse: SparseSet::with_capacity(self.capacity),
        }
    }
}

pub struct Table<I: Into<GenId> + Clone> {
    id: TableId,
    columns: ImmutableSparseSet<Column>,
    rows: Vec<I>,
    sparse: SparseSet<Row>,
}

impl<I: Into<GenId> + Clone> Table<I> {
    pub fn with_capacity(capacity: usize) -> TableBuilder<I> {
        TableBuilder::with_capacity(capacity)
    }

    pub fn from_row(row: &TableRow<I>, capacity: usize) -> Self {
        let mut columns = SparseSet::with_capacity(row.iter().count());

        for index in row.indices() {
            let column = row.column(index).unwrap().copy(capacity);
            columns.insert(index, column);
        }

        Self {
            id: TableId::new(&columns.indices().collect::<Vec<_>>()),
            columns: columns.into_immutable(),
            rows: Vec::with_capacity(capacity),
            sparse: SparseSet::with_capacity(capacity),
        }
    }

    pub fn id(&self) -> TableId {
        self.id
    }

    pub fn cell(&self, row: I, column: usize) -> Option<TableCell> {
        let gen_id: GenId = row.into();
        if let Some(row) = self.sparse.get(gen_id.id()) {
            self.columns
                .get(column)
                .and_then(|column| column.offset(**row))
                .map(TableCell::new)
        } else {
            None
        }
    }

    pub fn get<T>(&self, row: I, column: usize) -> Option<&T> {
        let gen_id: GenId = row.into();
        if let Some(row) = self.sparse.get(gen_id.id()) {
            self.columns
                .get(column)
                .and_then(|column| column.get(**row))
        } else {
            None
        }
    }

    pub fn get_mut<T>(&self, row: I, column: usize) -> Option<&mut T> {
        let gen_id: GenId = row.into();
        if let Some(row) = self.sparse.get(gen_id.id()) {
            self.columns
                .get(column)
                .and_then(|column| column.get_mut(**row))
        } else {
            None
        }
    }

    pub fn columns(&self) -> impl Iterator<Item = &Column> {
        self.columns.iter()
    }

    pub fn column(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }

    pub fn column_mut(&mut self, index: usize) -> Option<&mut Column> {
        self.columns.get_mut(index)
    }

    pub fn row(&self, row: I) -> Option<SelectedRow<I>> {
        self.select_row(row, &self.columns.indices().collect::<Vec<_>>())
    }

    pub fn row_index(&self, row: usize) -> Option<SelectedRow<I>> {
        self.row(self.rows.get(row)?.clone())
    }

    pub fn select_row(&self, row: I, columns: &[usize]) -> Option<SelectedRow<I>> {
        let gen_id: GenId = row.clone().into();
        if let Some(_row) = self.sparse.get(gen_id.id()) {
            let mut cells = SparseSet::with_capacity(columns.len());

            for &column in columns {
                if let Some(cell) = self
                    .columns
                    .get(column)
                    .and_then(|column| column.offset(**_row))
                {
                    cells.insert(column, TableCell::new(cell));
                }
            }

            Some(SelectedRow::new(row, cells.into_immutable()))
        } else {
            None
        }
    }

    pub fn remove_row(&mut self, row: I) -> Option<TableRow<I>> {
        let gen_id: GenId = row.clone().into();
        if let Some(_row) = self.sparse.remove(gen_id.id()) {
            let mut columns = SparseSet::with_capacity(self.columns.len());

            for index in &self.columns.indices().collect::<Vec<_>>() {
                let column = self.column_mut(*index).unwrap();
                let blob = column.swap_remove(*_row);
                let mut column = column.copy(1);
                column.push_blob(blob);
                columns.insert(*index, column);
            }

            self.rows.swap_remove(*_row);

            Some(TableRow::new(row, columns))
        } else {
            None
        }
    }

    pub fn add_row(&mut self, id: I, mut row: TableRow<I>) -> Row {
        let gen_id: GenId = id.clone().into();
        let new_row = Row::new(self.rows.len());
        self.sparse.insert(gen_id.id(), new_row);
        self.rows.push(id.clone());

        for index in &self.columns.indices().collect::<Vec<_>>() {
            let mut column = row.remove(*index).expect("Missing column");
            self.column_mut(*index)
                .unwrap()
                .push_blob(column.swap_remove(0));
        }

        new_row
    }

    pub fn capacity(&self) -> usize {
        self.rows.capacity()
    }

    pub fn rows(&self) -> &[I] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

pub struct TableCell<'a>(Ptr<'a>);

impl<'a> TableCell<'a> {
    pub fn new(ptr: Ptr<'a>) -> Self {
        Self(ptr)
    }

    pub fn get<T>(&self) -> &T {
        self.0.get(0)
    }

    pub fn get_mut<T>(&self) -> &mut T {
        self.0.get_mut(0)
    }
}

pub struct SelectedRow<'a, I: Into<GenId> + Clone> {
    id: I,
    cells: ImmutableSparseSet<TableCell<'a>>,
}

impl<'a, I: Into<GenId> + Clone> SelectedRow<'a, I> {
    pub fn new(id: I, cells: ImmutableSparseSet<TableCell<'a>>) -> Self {
        Self { id, cells }
    }

    pub fn id(&self) -> &I {
        &self.id
    }

    pub fn columns(&self) -> impl Iterator<Item = usize> + '_ {
        self.cells.indices()
    }

    pub fn cell(&self, column: usize) -> Option<&TableCell<'a>> {
        self.cells.get(column)
    }
}

pub struct TableRow<I: Into<GenId> + Clone> {
    id: I,
    columns: SparseSet<Column>,
}

impl<I: Into<GenId> + Clone> TableRow<I> {
    pub fn new(id: I, columns: SparseSet<Column>) -> Self {
        Self { id, columns }
    }

    pub fn id(&self) -> &I {
        &self.id
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.columns.indices()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Column> {
        self.columns.iter()
    }

    pub fn column(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }

    pub fn column_mut(&mut self, index: usize) -> Option<&mut Column> {
        self.columns.get_mut(index)
    }

    pub fn insert(&mut self, index: usize, column: Column) -> Option<Column> {
        self.columns.insert(index, column)
    }

    pub fn remove(&mut self, index: usize) -> Option<Column> {
        self.columns.remove(index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableId(u64);

impl TableId {
    pub fn new(columns: &[usize]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        columns.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl From<u64> for TableId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl std::ops::Deref for TableId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Tables<I: Into<GenId> + Clone> {
    tables: SparseMap<TableId, Table<I>>,
}

impl<I: Into<GenId> + Clone> Tables<I> {
    pub fn new() -> Self {
        Self {
            tables: SparseMap::new(),
        }
    }

    pub fn insert(&mut self, table: Table<I>) {
        self.tables.insert(table.id(), table);
    }

    pub fn get(&self, id: TableId) -> Option<&Table<I>> {
        self.tables.get(&id)
    }

    pub fn get_mut(&mut self, id: TableId) -> Option<&mut Table<I>> {
        self.tables.get_mut(&id)
    }

    pub fn array(&self, ids: &[TableId]) -> Box<[&Table<I>]> {
        let mut array = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(table) = self.tables.get(id) {
                array.push(table);
            }
        }

        array.into_boxed_slice()
    }
}
