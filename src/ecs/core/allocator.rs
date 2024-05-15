#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenId {
    id: usize,
    generation: u32,
}

impl GenId {
    pub fn new(id: usize, generation: u32) -> Self {
        Self { id, generation }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

pub struct IdAllocator {
    next_id: usize,
    free: Vec<usize>,
    generations: Vec<u32>,
}

impl IdAllocator {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            free: Vec::new(),
            generations: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> GenId {
        let id = if let Some(id) = self.free.pop() {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.generations.push(0);
            id
        };

        GenId::new(id, self.generations[id])
    }

    pub fn free(&mut self, id: GenId) {
        let index = id.id();
        self.generations[index] += 1;
        self.free.push(index);
    }

    pub fn free_list(&mut self, ids: impl Iterator<Item = GenId>) {
        for id in ids {
            self.free(id);
        }
    }

    pub fn reserve(&mut self, amount: usize) {
        let new_capacity = self.next_id + amount;

        if self.generations.capacity() < new_capacity {
            self.generations
                .reserve(new_capacity - self.generations.capacity());
        }

        if self.free.capacity() < new_capacity {
            self.free.reserve(new_capacity - self.free.capacity());
        }

        for index in new_capacity..self.free.len() {
            self.free.push(index);
        }

        self.next_id = new_capacity;
    }

    pub fn iter(&self) -> impl Iterator<Item = GenId> + '_ {
        self.generations
            .iter()
            .enumerate()
            .filter_map(|(id, generation)| {
                if *generation != 0 {
                    Some(GenId::new(id, *generation))
                } else {
                    None
                }
            })
    }

    pub fn contains(&self, id: GenId) -> bool {
        self.is_alive(id)
    }

    pub fn is_alive(&self, id: GenId) -> bool {
        id.id() < self.next_id && self.generations[id.id()] == id.generation()
    }

    pub fn is_empty(&self) -> bool {
        self.next_id == 0
    }

    pub fn len(&self) -> usize {
        self.next_id - self.free.len()
    }

    pub fn clear(&mut self) {
        self.next_id = 0;
        self.free.clear();
        self.generations.clear();
    }
}
