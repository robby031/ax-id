#[cfg(feature = "serde")]
pub(crate) mod serde_impl {
    use core::fmt;

    use serde::{Deserializer, Serializer, de::Visitor};

    use crate::Id;

    impl serde::Serialize for Id {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for Id {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct IdVisitor;

            impl<'de> Visitor<'de> for IdVisitor {
                type Value = Id;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("an unsigned 64-bit integer or hex string")
                }

                fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Id, E> {
                    Ok(Id(value))
                }

                fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Id, E> {
                    value.parse().map_err(serde::de::Error::custom)
                }
            }

            deserializer.deserialize_any(IdVisitor)
        }
    }

    pub mod hex {
        use core::fmt;

        use serde::{Deserializer, Serializer, de::Visitor};

        use crate::Id;

        pub fn serialize<S: Serializer>(id: &Id, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(&HexFormatter(id.0))
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Id, D::Error> {
            struct HexVisitor;

            impl<'de> Visitor<'de> for HexVisitor {
                type Value = Id;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a 16-character hex string")
                }

                fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Id, E> {
                    u64::from_str_radix(value, 16)
                        .map(Id)
                        .map_err(serde::de::Error::custom)
                }
            }

            deserializer.deserialize_str(HexVisitor)
        }

        struct HexFormatter(u64);

        impl fmt::Display for HexFormatter {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:016x}", self.0)
            }
        }
    }
}

#[cfg(feature = "bytemuck")]
mod bytemuck_impl {
    use bytemuck::{Pod, Zeroable};

    use crate::Id;

    unsafe impl Zeroable for Id {}
    unsafe impl Pod for Id {}
}

// zerocopy support is implemented via derive macros on the Id struct in id.rs

#[cfg(feature = "arbitrary")]
mod arbitrary_impl {
    use arbitrary::Arbitrary;

    use crate::Id;

    impl<'a> Arbitrary<'a> for Id {
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            u64::arbitrary(u).map(Id)
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            (8, Some(8))
        }
    }
}

// rkyv support is implemented via derive macros on the Id struct in id.rs

#[cfg(feature = "borsh")]
mod borsh_impl {
    use borsh::{BorshDeserialize, BorshSerialize, io};

    use crate::Id;

    impl BorshSerialize for Id {
        fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
            self.0.serialize(writer)
        }
    }

    impl BorshDeserialize for Id {
        fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
            u64::deserialize_reader(reader).map(Id)
        }
    }
}

#[cfg(feature = "sqlx")]
mod sqlx_impl {
    use sqlx::{Database as Db, database::Database, decode::Decode, encode::Encode, types::Type};

    use crate::Id;

    impl<DB: Database> Type<DB> for Id
    where
        i64: Type<DB>,
    {
        fn type_info() -> <DB as Db>::TypeInfo {
            <i64 as Type<DB>>::type_info()
        }
    }

    impl<'q, DB: Database> Encode<'q, DB> for Id
    where
        i64: Encode<'q, DB>,
    {
        fn encode(
            self,
            buf: &mut <DB as Db>::ArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <i64 as Encode<DB>>::encode(self.0 as i64, buf)
        }

        fn encode_by_ref(
            &self,
            buf: &mut <DB as Db>::ArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            <i64 as Encode<DB>>::encode_by_ref(&(self.0 as i64), buf)
        }
    }

    impl<'r, DB: Database> Decode<'r, DB> for Id
    where
        i64: Decode<'r, DB>,
    {
        fn decode(value: <DB as Db>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
            <i64 as Decode<DB>>::decode(value).map(|v| Id(v as u64))
        }
    }
}

#[cfg(feature = "diesel")]
mod diesel_impl {
    use diesel::{
        backend::Backend,
        deserialize::FromSql,
        expression::AsExpression,
        serialize::{Output, ToSql},
        sql_types::BigInt,
    };

    use crate::Id;

    impl<DB: Backend> FromSql<BigInt, DB> for Id
    where
        i64: FromSql<BigInt, DB>,
    {
        fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
            i64::from_sql(bytes).map(|v| Id(v as u64))
        }
    }

    impl<DB: Backend> ToSql<BigInt, DB> for Id
    where
        i64: ToSql<BigInt, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
            let val: &'b i64 = unsafe { &*(&self.0 as *const u64).cast::<i64>() };
            <i64 as ToSql<BigInt, DB>>::to_sql(val, out)
        }
    }

    impl AsExpression<BigInt> for Id {
        type Expression = <i64 as AsExpression<BigInt>>::Expression;

        fn as_expression(self) -> Self::Expression {
            <i64 as AsExpression<BigInt>>::as_expression(self.0 as i64)
        }
    }
}

#[cfg(feature = "sea-orm")]
mod sea_orm_impl {
    use sea_orm::{
        ColIdx, DbErr, FromQueryResult, QueryResult, TryFromU64, TryGetError, TryGetable, Value,
    };

    use crate::Id;

    impl TryFromU64 for Id {
        fn try_from_u64(n: u64) -> Result<Self, DbErr> {
            Ok(Id(n))
        }
    }

    impl TryGetable for Id {
        fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
            let val: i64 = res.try_get_by(index)?;
            Ok(Id(val as u64))
        }

        fn try_get(res: &QueryResult, pre: &str, col: &str) -> Result<Self, TryGetError> {
            let val: i64 = res.try_get(pre, col)?;
            Ok(Id(val as u64))
        }
    }

    impl FromQueryResult for Id {
        fn from_query_result(res: &QueryResult, pre: &str) -> Result<Self, DbErr> {
            let val: i64 = res.try_get(pre, "id")?;
            Ok(Id(val as u64))
        }
    }

    impl From<Id> for Value {
        fn from(id: Id) -> Self {
            Value::BigUnsigned(Some(id.0))
        }
    }

    impl sea_orm::sea_query::Nullable for Id {
        fn null() -> Value {
            Value::BigUnsigned(None)
        }
    }
}
