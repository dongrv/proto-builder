//! Maintainer-only packaging tool. The shipped product remains one proto-builder.exe.
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Prepare signed proto-builder release assets")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Keygen {
        #[arg(long)]
        directory: PathBuf,
    },
    Pack {
        #[arg(long)]
        binary: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long,default_value="stable",value_parser=["stable","beta"])]
        channel: String,
    },
}
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Keygen { directory } => {
            if !cfg!(windows) {
                bail!("DPAPI key generation currently supports Windows only");
            }
            if directory.exists() {
                bail!(
                    "key directory already exists; refusing to overwrite {}",
                    directory.display()
                );
            }
            let mut secret = [0u8; 32];
            getrandom::getrandom(&mut secret).map_err(|e| anyhow::anyhow!("OS RNG: {e}"))?;
            let signing = SigningKey::from_bytes(&secret);
            let encrypted = protect(&secret)?;
            secret.fill(0);
            fs::create_dir_all(&directory)?;
            fs::write(directory.join("signing-key.dpapi"), encrypted)?;
            fs::write(
                directory.join("public-key.hex"),
                hex::encode(signing.verifying_key().to_bytes()) + "\n",
            )?;
            println!(
                "Private key (Windows current-user DPAPI): {}",
                directory.join("signing-key.dpapi").display()
            );
            println!("Public key: {}", directory.join("public-key.hex").display());
            Ok(())
        }
        Command::Pack {
            binary,
            key,
            output,
            version,
            channel,
        } => {
            let version = semver::Version::parse(version.trim_start_matches('v'))?;
            if channel == "stable" && !version.pre.is_empty() {
                bail!("stable releases cannot use prerelease versions");
            }
            if !output.exists() {
                fs::create_dir_all(&output)?;
            }
            let mut secret = unprotect(&fs::read(key)?)?;
            let mut raw: [u8; 32] = secret
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid private key length"))?;
            let signing = SigningKey::from_bytes(&raw);
            raw.fill(0);
            secret.fill(0);
            let public = hex::encode(signing.verifying_key().to_bytes());
            let check = std::process::Command::new(&binary)
                .args(["version", "--json", "--non-interactive"])
                .output()
                .context("run release executable")?;
            if !check.status.success() {
                bail!("release executable version check failed");
            }
            let info: serde_json::Value = serde_json::from_slice(&check.stdout)?;
            if info["version"] != version.to_string() {
                bail!("requested version differs from binary version");
            }
            if info["result"]["update_public_key"] != public {
                bail!("release key differs from public key embedded in the executable");
            }
            let bytes = fs::read(&binary)?;
            if !is_windows_x64(&bytes) {
                bail!("release asset must be a Windows x86_64 PE executable");
            }
            let name = format!("proto-builder-{version}-windows-x86_64.exe");
            let asset = output.join(&name);
            let manifest = serde_json::json!({"repository":"dongrv/proto-builder","version":version.to_string(),"channel":channel,"assets":[{"platform":"windows-x86_64","name":name,"sha256":hex::encode(Sha256::digest(&bytes)),"size":bytes.len()}]});
            let mut encoded = serde_json::to_vec_pretty(&manifest)?;
            encoded.push(b'\n');
            let signature = hex::encode(signing.sign(&encoded).to_bytes()) + "\n";
            create(&asset, &bytes)?;
            create(&output.join("release.json"), &encoded)?;
            create(&output.join("release.json.sig"), signature.as_bytes())?;
            let checksums = format!(
                "{}  {}\n{}  release.json\n{}  release.json.sig\n",
                hex::encode(Sha256::digest(&bytes)),
                name,
                hex::encode(Sha256::digest(&encoded)),
                hex::encode(Sha256::digest(signature.as_bytes()))
            );
            create(&output.join("SHA256SUMS"), checksums.as_bytes())?;
            create(&output.join("public-key.hex"), (public + "\n").as_bytes())?;
            println!("Signed release assets: {}", output.display());
            Ok(())
        }
    }
}
fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "create {} (never overwrite signed releases)",
                path.display()
            )
        })?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn is_windows_x64(bytes: &[u8]) -> bool {
    if bytes.len() < 64 || &bytes[..2] != b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    bytes
        .get(offset..offset + 6)
        .is_some_and(|v| &v[..4] == b"PE\0\0" && v[4..6] == [0x64, 0x86])
}
#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>> {
    crypt(bytes, true)
}
#[cfg(windows)]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>> {
    crypt(bytes, false)
}
#[cfg(windows)]
fn crypt(bytes: &[u8], protect: bool) -> Result<Vec<u8>> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    unsafe {
        let result = if protect {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error()).context("Windows current-user DPAPI");
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        Ok(result)
    }
}
#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>> {
    bail!("Windows DPAPI required")
}
#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>> {
    bail!("Windows DPAPI required")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn dpapi_round_trip() {
        let value = [19u8; 32];
        let encrypted = protect(&value).unwrap();
        assert_ne!(encrypted, value);
        assert_eq!(unprotect(&encrypted).unwrap(), value);
    }
}
