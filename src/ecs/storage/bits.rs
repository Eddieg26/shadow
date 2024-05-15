pub struct BitSet {
    bits: Vec<u8>,
}

impl BitSet {
    pub fn new() -> Self {
        Self { bits: vec![] }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: Vec::with_capacity(capacity),
        }
    }

    pub fn set(&mut self, index: usize) {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }

        self.bits[word] |= 1 << bit;
    }

    pub fn unset(&mut self, index: usize) {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }

        self.bits[word] &= !(1 << bit);
    }

    pub fn get(&self, index: usize) -> bool {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            return false;
        }

        self.bits[word] & (1 << bit) != 0
    }

    pub fn index(&self, index: usize) -> (usize, usize) {
        let byte = index / 8;
        let bit = index % 8;

        (byte, bit)
    }

    pub fn or(&self, other: &Self) -> BitSet {
        let len = self.len().max(other.len());
        let mut result = BitSet::with_capacity(len);

        for i in 0..len {
            if self.get(i) || other.get(i) {
                result.set(i);
            }
        }

        result
    }

    pub fn all_off(&self) -> bool {
        for word in self.bits.iter() {
            if *word != 0 {
                return false;
            }
        }

        true
    }

    pub fn len(&self) -> usize {
        self.bits.len() * 8
    }

    pub fn iter(&self) -> BitSetIter {
        BitSetIter::new(self)
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn clear(&mut self) {
        self.bits.clear();
    }
}

pub struct BitSetIter<'a> {
    set: &'a BitSet,
    index: usize,
}

impl<'a> BitSetIter<'a> {
    pub fn new(set: &'a BitSet) -> Self {
        Self { set, index: 0 }
    }
}

impl<'a> Iterator for BitSetIter<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.set.len() {
            return None;
        }

        let result = self.set.get(self.index);
        self.index += 1;

        Some(result)
    }
}
