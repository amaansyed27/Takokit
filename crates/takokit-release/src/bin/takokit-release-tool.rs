use std::{env, fs, path::PathBuf};
use takokit_release::{
    parse_manifest, parse_signature, sign_test_fixture, sign_with_seed, verify_signature,
    PRODUCTION_KEY_ID, TEST_KEY_ID,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("sign") => {
            let manifest = path(&mut args, "manifest")?;
            let signature = path(&mut args, "signature")?;
            let test = args.any(|arg| arg == "--test");
            let bytes = fs::read(&manifest)?;
            let parsed = parse_manifest(&bytes)?;
            let envelope = if test {
                if !parsed.test_fixture || parsed.channel != "test" || parsed.signing_key_id != TEST_KEY_ID {
                    return Err("--test may sign only an explicitly marked test fixture".into());
                }
                sign_test_fixture(&bytes)?
            } else {
                if parsed.test_fixture || parsed.signing_key_id != PRODUCTION_KEY_ID {
                    return Err("production signing requires production manifest metadata".into());
                }
                let seed = env::var("TAKOKIT_RELEASE_SIGNING_KEY_HEX")
                    .map_err(|_| "TAKOKIT_RELEASE_SIGNING_KEY_HEX is required for production signing")?;
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
            let parsed = parse_manifest(&bytes)?;
            println!("verified {} {} with {}", parsed.product, parsed.version, envelope.key_id);
        }
        _ => {
            eprintln!("usage: takokit-release-tool sign <manifest> <signature> [--test]");
            eprintln!("       takokit-release-tool verify <manifest> <signature> [--allow-test]");
            std::process::exit(2);
        }
    }
    Ok(())
}

fn path(args: &mut impl Iterator<Item = String>, label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {label} path").into())
}
