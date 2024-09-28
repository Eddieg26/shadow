use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenId {
    id: u32,
    gen: u32,
}

impl GenId {
    pub fn new(id: u32, gen: u32) -> GenId {
        GenId { id, gen }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn gen(&self) -> u32 {
        self.gen
    }
}

impl std::ops::Deref for GenId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

pub struct Allocator {
    current: u32,
    generations: HashMap<u32, u32>,
    free: Vec<u32>,
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

    pub fn free(&mut self, id: &GenId) -> bool {
        if let Some(gen) = self.generations.get(id) {
            if *gen == id.gen {
                self.free.push(**id);
                self.generations.insert(**id, gen + 1);
                return true;
            }
        }

        false
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.generations.iter().map(|(id, gen)| (*id, *gen))
    }
}
