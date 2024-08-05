use shadow_ecs::core::{DenseMap, DenseSet};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    hash::Hash,
    path::PathBuf,
};

pub trait IntoBytes: Sized {
    fn into_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}
impl IntoBytes for () {
    fn into_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_bytes: &[u8]) -> Option<Self> {
        Some(())
    }
}

impl IntoBytes for usize {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for u8 {
    fn into_bytes(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0])
    }
}

impl IntoBytes for u16 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl IntoBytes for u32 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 4];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for u64 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for u128 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 16];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for i8 {
    fn into_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] as i8)
    }
}

impl IntoBytes for i16 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl IntoBytes for i32 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 4];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for i64 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for i128 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 16];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for f32 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 4];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for f64 {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut buf = [0; 8];
        buf.copy_from_slice(&bytes);
        Some(Self::from_le_bytes(buf))
    }
}

impl IntoBytes for bool {
    fn into_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] != 0)
    }
}

impl IntoBytes for char {
    fn into_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(bytes).unwrap();
        s.chars().next()
    }
}

impl IntoBytes for String {
    fn into_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = std::str::from_utf8(bytes).ok()?;
        Some(value.to_string())
    }
}

impl<T: IntoBytes> IntoBytes for Vec<T> {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let len = self.len() as u32;
        bytes.extend_from_slice(&len.into_bytes());
        for item in self {
            let item_bytes = item.into_bytes();
            let len = item_bytes.len() as u32;
            bytes.extend_from_slice(&len.into_bytes());
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
            offset += item.into_bytes().len() as usize + 4;
            items.push(item);
        }
        Some(items)
    }
}

impl<T: IntoBytes + Eq + Hash> IntoBytes for HashSet<T> {
    fn into_bytes(&self) -> Vec<u8> {
        self.iter()
            .map(|item| item.into_bytes())
            .collect::<Vec<_>>()
            .into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let items = Vec::<T>::from_bytes(bytes)?;
        let set = items.into_iter().collect();
        Some(set)
    }
}

impl<K: IntoBytes + Eq + Hash, V: IntoBytes> IntoBytes for HashMap<K, V> {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let len = self.len();
        bytes.extend_from_slice(&len.into_bytes());
        for (key, value) in self.iter() {
            let key_bytes = key.into_bytes();
            let key_len = key_bytes.len();
            bytes.extend_from_slice(&key_len.into_bytes());
            bytes.extend_from_slice(&key_bytes);

            let value_bytes = value.into_bytes();
            let value_len = value_bytes.len();
            bytes.extend_from_slice(&value_len.into_bytes());
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

impl<K: IntoBytes + Eq + Hash + Clone, V: IntoBytes> IntoBytes for DenseMap<K, V> {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let len = self.len();
        bytes.extend_from_slice(&len.into_bytes());

        for (key, value) in self.iter() {
            let key_bytes = key.into_bytes();
            let key_len = key_bytes.len();
            bytes.extend_from_slice(&key_len.into_bytes());
            bytes.extend_from_slice(&key_bytes);

            let value_bytes = value.into_bytes();
            let value_len = value_bytes.len();
            bytes.extend_from_slice(&value_len.into_bytes());
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

impl<K: Clone + Hash + Eq + IntoBytes> IntoBytes for DenseSet<K> {
    fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        let len = self.len();
        bytes.extend_from_slice(&len.into_bytes());

        for key in self.iter() {
            let key_bytes = key.into_bytes();
            let key_len = key_bytes.len();
            bytes.extend_from_slice(&key_len.into_bytes());
            bytes.extend_from_slice(&key_bytes);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut set = DenseSet::new();
        let mut offset = 0;

        let len = usize::from_bytes(&bytes[offset..offset + 8])?;
        offset += 8;
        for _ in 0..len {
            let key_len = usize::from_bytes(&bytes[offset..offset + 8])?;
            offset += 8;
            let key = K::from_bytes(&bytes[offset..offset + key_len])?;
            offset += key_len;

            set.insert(key);
        }

        Some(set)
    }
}

impl<T: IntoBytes> IntoBytes for Option<T> {
    fn into_bytes(&self) -> Vec<u8> {
        match self {
            Some(value) => {
                let mut bytes = vec![1];
                let item_bytes = value.into_bytes();
                let len = item_bytes.len() as u32;
                bytes.extend_from_slice(&len.into_bytes());
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

impl<T: IntoBytes> IntoBytes for Box<T> {
    fn into_bytes(&self) -> Vec<u8> {
        self.as_ref().into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = T::from_bytes(bytes)?;
        Some(Box::new(value))
    }
}

impl IntoBytes for OsString {
    fn into_bytes(&self) -> Vec<u8> {
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

impl IntoBytes for PathBuf {
    fn into_bytes(&self) -> Vec<u8> {
        self.clone().into_os_string().into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        OsString::from_bytes(bytes).map(PathBuf::from)
    }
}
