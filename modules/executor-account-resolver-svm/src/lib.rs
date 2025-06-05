pub const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];

#[cfg(test)]
mod test {
    use super::*;
    use solana_sha256_hasher::hashv;
    //
    #[test]
    fn test_resolver_discriminators_match() {
        // https://github.com/solana-program/libraries/blob/fcd6052feccb74b5ae4f7a8a7858e85d7f4adc93/discriminator/src/discriminator.rs#L40-L42
        let hash_input = "executor-account-resolver:execute-vaa-v1";
        let hash_bytes = hashv(&[hash_input.as_bytes()]).to_bytes();
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
        assert_eq!(RESOLVER_EXECUTE_VAA_V1, discriminator_bytes);
    }
}
