use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    hash::Hash,
    path::PathBuf,
};

use shadow_ecs::ecs::storage::dense::DenseMap;

pub trait ToBytes: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl ToBytes for () {
    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_bytes: &[u8]) -> Option<Self> {
        Some(())
    }
}

impl ToBytes for usize {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl ToBytes for u8 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0])
    }
}

impl ToBytes for u16 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl ToBytes for u32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl ToBytes for u64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl ToBytes for u128 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]))
    }
}

impl ToBytes for i8 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] as i8)
    }
}

impl ToBytes for i16 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl ToBytes for i32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl ToBytes for i64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl ToBytes for i128 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]))
    }
}

impl ToBytes for f32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl ToBytes for f64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl ToBytes for bool {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] != 0)
    }
}

impl ToBytes for char {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(bytes).unwrap();
        s.chars().next()
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = std::str::from_utf8(bytes).ok()?;
        Some(value.to_string())
    }
}

impl<T: ToBytes> ToBytes for Vec<T> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let len = self.len() as u32;
        bytes.extend_from_slice(&len.to_bytes());
        for item in self {
            let item_bytes = item.to_bytes();
            let len = item_bytes.len() as u32;
            bytes.extend_from_slice(&len.to_bytes());
            bytes.extend_from_slice(&item_bytes);
        }
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut items = vec![];
        let mut offset = 0;
        let len = u32::from_bytes(&bytes[offset..offset + 4])?;
        offset += 4;
        for _ in 0..len {
            let len = u32::from_bytes(&bytes[offset..offset + 4])?;
            let item = T::from_bytes(&bytes[offset + 4..offset + 4 + len as usize])?;
            offset += item.to_bytes().len() as usize + 4;
            items.push(item);
        }
        Some(items)
    }
}

impl<T: ToBytes + Eq + Hash> ToBytes for HashSet<T> {
    fn to_bytes(&self) -> Vec<u8> {
        self.iter()
            .map(|item| item.to_bytes())
            .collect::<Vec<_>>()
            .to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let items = Vec::<T>::from_bytes(bytes)?;
        let set = items.into_iter().collect();
        Some(set)
    }
}

impl<K: ToBytes + Eq + Hash, V: ToBytes> ToBytes for HashMap<K, V> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let len = self.len();
        bytes.extend_from_slice(&len.to_bytes());
        for (key, value) in self.iter() {
            let key_bytes = key.to_bytes();
            let key_len = key_bytes.len();
            bytes.extend_from_slice(&key_len.to_bytes());
            bytes.extend_from_slice(&key_bytes);

            let value_bytes = value.to_bytes();
            let value_len = value_bytes.len();
            bytes.extend_from_slice(&value_len.to_bytes());
            bytes.extend_from_slice(&value_bytes);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut map = HashMap::new();
        let mut offset = 0;

        let len = usize::from_bytes(&bytes[offset..offset + 8])?;
        offset += 8;
        for _ in 0..len {
            let key_len = usize::from_bytes(&bytes[offset..offset + 8])?;
            offset += 8;
            let key = K::from_bytes(&bytes[offset..offset + key_len])?;
            offset += key_len;

            let value_len = usize::from_bytes(&bytes[offset..offset + 8])?;
            offset += 8;
            let value = V::from_bytes(&bytes[offset..offset + value_len])?;
            offset += value_len;

            map.insert(key, value);
        }

        Some(map)
    }
}

impl<K: ToBytes + Eq + Hash, V: ToBytes> ToBytes for DenseMap<K, V> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let len = self.len();
        bytes.extend_from_slice(&len.to_bytes());

        for (key, value) in self.iter() {
            let key_bytes = key.to_bytes();
            let key_len = key_bytes.len();
            bytes.extend_from_slice(&key_len.to_bytes());
            bytes.extend_from_slice(&key_bytes);

            let value_bytes = value.to_bytes();
            let value_len = value_bytes.len();
            bytes.extend_from_slice(&value_len.to_bytes());
            bytes.extend_from_slice(&value_bytes);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut map = DenseMap::new();
        let mut offset = 0;

        let len = usize::from_bytes(&bytes[offset..offset + 8])?;
        offset += 8;
        for _ in 0..len {
            let key_len = usize::from_bytes(&bytes[offset..offset + 8])?;
            offset += 8;
            let key = K::from_bytes(&bytes[offset..offset + key_len])?;
            offset += key_len;

            let value_len = usize::from_bytes(&bytes[offset..offset + 8])?;
            offset += 8;
            let value = V::from_bytes(&bytes[offset..offset + value_len])?;
            offset += value_len;

            map.insert(key, value);
        }

        Some(map)
    }
}

impl<T: ToBytes> ToBytes for Option<T> {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Some(value) => {
                let mut bytes = vec![1];
                let item_bytes = value.to_bytes();
                let len = item_bytes.len() as u32;
                bytes.extend_from_slice(&len.to_bytes());
                bytes.extend_from_slice(&item_bytes);
                bytes
            }
            None => vec![0],
        }
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes[0] == 0 {
            None
        } else {
            let len = u32::from_bytes(&bytes[1..5])?;
            let item = T::from_bytes(&bytes[5..5 + len as usize]);
            Some(item)
        }
    }
}

impl<T: ToBytes> ToBytes for Box<T> {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_ref().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = T::from_bytes(bytes)?;
        Some(Box::new(value))
    }
}

impl ToBytes for OsString {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_encoded_bytes().iter().copied().collect::<Vec<_>>()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        unsafe {
            Some(Self::from_encoded_bytes_unchecked(
                bytes.iter().copied().collect::<Vec<_>>(),
            ))
        }
    }
}

impl ToBytes for PathBuf {
    fn to_bytes(&self) -> Vec<u8> {
        self.clone().into_os_string().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        OsString::from_bytes(bytes).map(PathBuf::from)
    }
}
