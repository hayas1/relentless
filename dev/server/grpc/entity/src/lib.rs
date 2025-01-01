pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("file_descriptor");

pub mod helloworld_pb {
    tonic::include_proto!("helloworld");
}

pub mod counter_pb {
    tonic::include_proto!("counter");

    impl From<num::bigint::Sign> for Sign {
        fn from(sign: num::bigint::Sign) -> Self {
            match sign {
                num::bigint::Sign::NoSign => Sign::NoSign,
                num::bigint::Sign::Plus => Sign::Plus,
                num::bigint::Sign::Minus => Sign::Minus,
            }
        }
    }
    impl From<Sign> for num::bigint::Sign {
        fn from(sign: Sign) -> Self {
            match sign {
                Sign::NoSign => num::bigint::Sign::NoSign,
                Sign::Plus => num::bigint::Sign::Plus,
                Sign::Minus => num::bigint::Sign::Minus,
            }
        }
    }
    impl From<num::BigInt> for BigInt {
        fn from(value: num::BigInt) -> Self {
            let (sign, repr) = value.to_bytes_be();
            BigInt { sign: Sign::from(sign).into(), repr }
        }
    }
    impl From<BigInt> for num::BigInt {
        fn from(value: BigInt) -> Self {
            let (sign, repr) = (value.sign(), value.repr);
            num::BigInt::from_bytes_be(sign.into(), &repr)
        }
    }
}

pub mod echo_pb {
    tonic::include_proto!("echo");
}
