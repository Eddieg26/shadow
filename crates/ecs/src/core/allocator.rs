use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenId {
    id: usize,
    gen: usize,
}

impl GenId {
    pub fn new(id: usize, gen: usize) -> GenId {
        GenId { id, gen }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn gen(&self) -> usize {
        self.gen
    }
}

impl std::ops::Deref for GenId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

pub struct Allocator {
    current: usize,
    generations: HashMap<usize, usize>,
    free: Vec<usize>,
}

impl Allocator {
    pub fn new() -> Allocator {
        Allocator {
            current: 0,
            generations: HashMap::new(),
            free: Vec::new(),
        }
    }

    pub fn allocate(&mut self) -> GenId {
        let id = if let Some(id) = self.free.pop() {
            id
        } else {
            let id = self.current;
            self.current += 1;
            self.generations.insert(id, 0);
            id
        };

        let gen = self.generations.entry(id).or_default();
        return GenId { id, gen: *gen };
    }

    pub fn free(&mut self, id: &GenId) {
        if let Some(gen) = self.generations.get(id) {
            if *gen == id.gen {
                self.free.push(**id);
                self.generations.insert(**id, gen + 1);
            }
        }
    }
}
