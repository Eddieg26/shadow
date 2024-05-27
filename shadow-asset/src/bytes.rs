pub trait AsBytes: Sized {
    fn as_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl AsBytes for () {
    fn as_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn from_bytes(_bytes: &[u8]) -> Option<Self> {
        Some(())
    }
}

impl AsBytes for u8 {
    fn as_bytes(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0])
    }
}

impl AsBytes for u16 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl AsBytes for u32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl AsBytes for u64 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl AsBytes for u128 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]))
    }
}

impl AsBytes for i8 {
    fn as_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] as i8)
    }
}

impl AsBytes for i16 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([bytes[0], bytes[1]]))
    }
}

impl AsBytes for i32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl AsBytes for i64 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl AsBytes for i128 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]))
    }
}

impl AsBytes for f32 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))
    }
}

impl AsBytes for f64 {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl AsBytes for bool {
    fn as_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] != 0)
    }
}

impl AsBytes for char {
    fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(bytes).unwrap();
        s.chars().next()
    }
}

impl AsBytes for String {
    fn as_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = std::str::from_utf8(bytes).ok()?;
        Some(value.to_string())
    }
}

impl<T: AsBytes> AsBytes for Vec<T> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let len = self.len() as u32;
        bytes.extend_from_slice(&len.as_bytes());
        for item in self {
            let item_bytes = item.as_bytes();
            let len = item_bytes.len() as u32;
            bytes.extend_from_slice(&len.as_bytes());
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
            offset += item.as_bytes().len() as usize + 4;
            items.push(item);
        }
        Some(items)
    }
}

impl<T: AsBytes> AsBytes for Option<T> {
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            Some(value) => {
                let mut bytes = vec![1];
                let item_bytes = value.as_bytes();
                let len = item_bytes.len() as u32;
                bytes.extend_from_slice(&len.as_bytes());
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

impl<T: AsBytes> AsBytes for Box<T> {
    fn as_bytes(&self) -> Vec<u8> {
        self.as_ref().as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value = T::from_bytes(bytes)?;
        Some(Box::new(value))
    }
}
