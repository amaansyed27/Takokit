use std::{env, fs, path::PathBuf};
use takokit_release::{
    parse_manifest, parse_signature, sign_test_fixture, sign_with_seed, verify_signature,
    ReleaseIndex, PRODUCTION_KEY_ID, TEST_KEY_ID,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("sign") => {
            let manifest = path(&mut args, "manifest")?;
            let signature = path(&mut args, "signature")?;
            let test = args.any(|arg| arg == "--test");
            let bytes = fs::read(&manifest)?;
            let parsed = signing_metadata(&bytes)?;
            let envelope = if test {
                if !parsed.test_fixture || parsed.channel != "test" || parsed.key_id != TEST_KEY_ID
                {
                    return Err("--test may sign only an explicitly marked test fixture".into());
                }
                sign_test_fixture(&bytes)?
            } else {
                if parsed.test_fixture || parsed.key_id != PRODUCTION_KEY_ID {
                    return Err("production signing requires production manifest metadata".into());
                }
                let seed = env::var("TAKOKIT_RELEASE_SIGNING_KEY_HEX").map_err(|_| {
                    "TAKOKIT_RELEASE_SIGNING_KEY_HEX is required for production signing"
                })?;
                sign_with_seed(&bytes, &seed, PRODUCTION_KEY_ID)?
            };
            fs::write(signature, serde_json::to_vec_pretty(&envelope)?)?;
        }
        Some("verify") => {
            let manifest = path(&mut args, "manifest")?;
            let signature = path(&mut args, "signature")?;
            let allow_test = args.any(|arg| arg == "--allow-test");
            let bytes = fs::read(manifest)?;
            let envelope = parse_signature(&fs::read(signature)?)?;
            verify_signature(&bytes, &envelope, allow_test)?;
            let parsed = signing_metadata(&bytes)?;
            println!(
                "verified {} {} with {}",
                parsed.product, parsed.version, envelope.key_id
            );
        }
        _ => {
            eprintln!("usage: takokit-release-tool sign <manifest> <signature> [--test]");
            eprintln!("       takokit-release-tool verify <manifest> <signature> [--allow-test]");
            std::process::exit(2);
        }
    }
    Ok(())
}

struct SigningMetadata {
    product: String,
    version: String,
    channel: String,
    key_id: String,
    test_fixture: bool,
}

fn signing_metadata(bytes: &[u8]) -> Result<SigningMetadata, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    if value.get("platforms").is_some() {
        let index: ReleaseIndex = serde_json::from_value(value)?;
        return Ok(SigningMetadata {
            product: index.product,
            version: index.version,
            channel: index.channel,
            key_id: index.signing_key_id,
            test_fixture: index.test_fixture,
        });
    }
    let manifest = parse_manifest(bytes)?;
    Ok(SigningMetadata {
        product: manifest.product,
        version: manifest.version,
        channel: manifest.channel,
        key_id: manifest.signing_key_id,
        test_fixture: manifest.test_fixture,
    })
}

fn path(
    args: &mut impl Iterator<Item = String>,
    label: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {label} path").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_index_exposes_signing_metadata() {
        let bytes = br#"{
          "schema_version": 1,
          "product": "Takokit",
          "version": "0.3.0",
          "channel": "test",
          "commit_sha": "abc",
          "signing_key_id": "takokit-test-fixture-v1",
          "test_fixture": true,
          "platforms": {}
        }"#;
        let metadata = signing_metadata(bytes).unwrap();
        assert_eq!(metadata.product, "Takokit");
        assert_eq!(metadata.version, "0.3.0");
        assert_eq!(metadata.channel, "test");
        assert_eq!(metadata.key_id, TEST_KEY_ID);
        assert!(metadata.test_fixture);
    }
}
