use crate::solidity::Address;
use crate::solidity::ConcreteSolidityType;
use crate::solidity::Function;
use crate::solidity::IntoType;
use crate::solidity::SolidityType;
use byteorder::{BigEndian, ByteOrder};
use sha3::{Digest, Keccak256};
use std::convert::TryInto;

pub struct Builder<'a> {
    name: Option<String>,
    pub(super) params: Vec<ConcreteSolidityType<'a>>,
}

impl<'a> Builder<'a> {
    pub fn new() -> Self {
        Builder {
            name: None,
            params: Vec::new(),
        }
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn add<F: IntoType<'a>>(mut self, value: F) -> Self {
        self.params.push(value.into_type());
        self
    }

    pub fn add_address<F: TryInto<Address>>(mut self, value: F) -> Result<Self, F::Error> {
        let address: Address = value.try_into()?;
        self.params.push(ConcreteSolidityType::Address(
            SolidityType::Address,
            address,
        ));

        Ok(self)
    }

    pub fn add_function<F: TryInto<Function>>(mut self, value: F) -> Result<Self, F::Error> {
        let function: Function = value.try_into()?;
        self.params.push(ConcreteSolidityType::Function(
            SolidityType::Function,
            function,
        ));

        Ok(self)
    }

    pub fn signature(&self) -> [u8; 4] {
        if let Some(name) = &self.name {
            let mut sig = [0; 4];
            let mut hasher = Keccak256::new();
            let function = format!(
                "{}({})",
                name,
                self.params
                    .iter()
                    .map(ConcreteSolidityType::to_string)
                    .collect::<Vec<String>>()
                    .join(",")
            );
            hasher.input(&function);
            sig.copy_from_slice(&hasher.result());
            sig
        } else {
            panic!("cannot calculate function signature without a name");
        }
    }

    pub fn build(self) -> Vec<u8> {
        let name_offset = match self.name {
            None => 0,
            Some(_) => 4,
        };

        let sig = if let Some(_) = self.name {
            Some(self.signature())
        } else {
            None
        };

        let total_len = self
            .params
            .iter()
            .map(ConcreteSolidityType::required_byte_len)
            .zip(self.params.iter().map(ConcreteSolidityType::is_dynamic))
            .fold(
                0,
                |sum, (len, dynamic)| if dynamic { 32 + sum + len } else { sum + len },
            );

        let mut buf: Vec<u8> = vec![0; total_len + name_offset];

        let mut offset: usize = self.params.len() * 32 + name_offset;

        for (index, (dynamic, bytes)) in self
            .params
            .into_iter()
            .map(ConcreteSolidityType::to_bytes)
            .into_iter()
            .enumerate()
        {
            if dynamic {
                BigEndian::write_u64(
                    &mut buf[index * 32 + 24 + name_offset..(index + 1) * 32 + name_offset],
                    offset as u64,
                );
                buf[offset..offset + bytes.len()].copy_from_slice(&bytes);
                offset += bytes.len()
            } else {
                buf[index * 32 + name_offset..(index + 1) * 32 + name_offset]
                    .copy_from_slice(&bytes);
            }
        }

        if let Some(sig) = sig {
            buf.copy_from_slice(&sig)
        }

        buf
    }
}

// This macro is used to generate all the `Builder::add_*()` methods for the various number types.
#[macro_use]
macro_rules! impl_solidity_function_for_builder {
    ($ty: ty => $solidity: ident: $function: ident | $array: ident) => {
        impl<'a> Builder<'a> {
            pub fn $function(mut self, value: $ty) -> Self {
                self.params.push(ConcreteSolidityType::$solidity(
                    SolidityType::$solidity,
                    value,
                ));
                self
            }

            pub fn $array(mut self, value: &Vec<$ty>) -> Self {
                use crate::solidity::SolidityArray;
                let array = value
                    .iter()
                    .map(|value| ConcreteSolidityType::$solidity(SolidityType::$solidity, *value))
                    .collect();

                self.params.push(ConcreteSolidityType::Array(
                    SolidityType::$solidity,
                    SolidityArray {
                        dimensions: 1,
                        array,
                    },
                ));
                self
            }
        }
    };
}

impl_solidity_function_for_builder!(i8 => I8: add_i8 | add_i8_array);
impl_solidity_function_for_builder!(u8 => U8: add_u8 | add_u8_array);
impl_solidity_function_for_builder!(i16 => I16: add_i16 | add_i16_array);
impl_solidity_function_for_builder!(u16 => U16 : add_u16 | add_u16_array);
impl_solidity_function_for_builder!(i32 => I32 : add_i32 | add_i32_array);
impl_solidity_function_for_builder!(u32 => U32 : add_u32 | add_u32_array);
impl_solidity_function_for_builder!(i64 => I64 : add_i64 | add_i64_array);
impl_solidity_function_for_builder!(u64 => U64 : add_u64 | add_u64_array);
impl_solidity_function_for_builder!(i128 => I128: add_i128 | add_i128_array);
impl_solidity_function_for_builder!(u128 => U128: add_u128 | add_u128_array);
impl_solidity_function_for_builder!(&'a [u8; 32] => I256: add_i256 | add_i256_array);
impl_solidity_function_for_builder!(&'a str => String: add_string | add_string_array);
impl_solidity_function_for_builder!(&'a [u8] => Bytes: add_bytes | add_bytes_array);
impl_solidity_function_for_builder!(&'a [u8; 32] => U256: add_u256 | add_u256_array);
