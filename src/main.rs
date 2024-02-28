use im::OrdMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct S {
    #[serde(with = "serde_ord_map")]
    map: OrdMap<(u8, u8), u8>,
}

fn main() -> anyhow::Result<()> {
    let old = S {
        map: OrdMap::from_iter(vec![((1, 2), 3), ((4, 5), 6)]),
    };
    println!("{old:?}");
    let json = serde_json::to_string(&old)?;
    println!("{json}");
    let new: S = serde_json::from_str(&json)?;
    println!("{new:?}");
    assert_eq!(old, new);
    Ok(())
}

mod serde_ord_map {
    use std::{convert::Infallible, marker::PhantomData};

    use anyhow::bail;
    use im::OrdMap;
    use serde::{
        de::{MapAccess, Visitor},
        ser::SerializeMap,
        Deserialize, Deserializer, Serialize, Serializer,
    };

    pub fn serialize<K, V, S>(value: &OrdMap<K, V>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Ord + SerializeKey,
        V: Serialize,
    {
        let mut map = ser.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            let Ok(key) = key.serialize_key() else {
                unreachable!();
            };
            map.serialize_entry(&key, &value)?;
        }
        map.end()
    }

    pub fn deserialize<'de, K, V, D>(de: D) -> Result<OrdMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Clone + Ord + DeserializeKey,
        V: Clone + Deserialize<'de>,
    {
        de.deserialize_map(MyVisitor(PhantomData, PhantomData))
    }

    struct MyVisitor<K, V>(PhantomData<fn() -> K>, PhantomData<fn() -> V>);
    impl<'de, K, V> Visitor<'de> for MyVisitor<K, V>
    where
        K: Clone + Ord + DeserializeKey,
        V: Clone + Deserialize<'de>,
    {
        type Value = OrdMap<K, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut v: Vec<(K, V)> = match map.size_hint() {
                None => Vec::new(),
                Some(l) => Vec::with_capacity(l),
            };
            while let Some(key) = map.next_key::<String>()? {
                let value = map.next_value()?;
                v.push((
                    K::deserialize_key(&key).map_err(serde::de::Error::custom)?,
                    value,
                ));
            }
            Ok(OrdMap::from(v))
        }
    }

    pub trait SerializeKey {
        type Error;
        fn serialize_key(&self) -> Result<String, Self::Error>;
    }

    pub trait DeserializeKey: Sized {
        type Error: std::fmt::Display;
        fn deserialize_key(s: &str) -> Result<Self, Self::Error>;
    }

    impl SerializeKey for (u8, u8) {
        type Error = Infallible;
        fn serialize_key(&self) -> Result<String, Infallible> {
            Ok(format!("{},{}", self.0, self.1))
        }
    }

    impl DeserializeKey for (u8, u8) {
        type Error = anyhow::Error;
        fn deserialize_key(s: &str) -> anyhow::Result<Self> {
            let Some((x, y)) = s.split_once(',') else {
                bail!("Key does not contain a comma");
            };
            Ok((x.parse()?, y.parse()?))
        }
    }
}
