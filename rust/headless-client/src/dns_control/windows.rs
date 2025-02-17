//! Gives Firezone DNS privilege over other DNS resolvers on the system
//!
//! This uses NRPT and claims all domains, similar to the `systemd-resolved` control method
//! on Linux.
//! This allows us to "shadow" DNS resolvers that are configured by the user or DHCP on
//! physical interfaces, as long as they don't have any NRPT rules that outrank us.
//!
//! If Firezone crashes, restarting Firezone and closing it gracefully will resume
//! normal DNS operation. The Powershell command to remove the NRPT rule can also be run
//! by hand.
//!
//! The system default resolvers don't need to be reverted because they're never deleted.
//!
//! <https://superuser.com/a/1752670>

use anyhow::{Context as _, Result};
use connlib_shared::windows::CREATE_NO_WINDOW;
use std::{net::IpAddr, os::windows::process::CommandExt, path::Path, process::Command};

pub fn system_resolvers_for_gui() -> Result<Vec<IpAddr>> {
    system_resolvers()
}

/// Controls system-wide DNS.
///
/// Always call `deactivate` when Firezone starts.
///
/// Only one of these should exist on the entire system at a time.
#[derive(Default)]
pub(crate) struct DnsController {}

// Unique magic number that we can use to delete our well-known NRPT rule.
// Copied from the deep link schema
const FZ_MAGIC: &str = "firezone-fd0020211111";

impl Drop for DnsController {
    fn drop(&mut self) {
        if let Err(error) = self.deactivate() {
            tracing::error!(?error, "Failed to deactivate DNS control");
        }
    }
}

impl DnsController {
    /// Deactivate any control Firezone has over the computer's DNS
    ///
    /// Must be `sync` so we can call it from `Drop`
    /// TODO: Replace this with manual registry twiddling one day if we feel safe.
    pub(crate) fn deactivate(&mut self) -> Result<()> {
        Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-Command", "Get-DnsClientNrptRule", "|"])
            .args(["where", "Comment", "-eq", FZ_MAGIC, "|"])
            .args(["foreach", "{"])
            .args(["Remove-DnsClientNrptRule", "-Name", "$_.Name", "-Force"])
            .args(["}"])
            .status()?;
        tracing::info!("Deactivated DNS control");
        Ok(())
    }

    /// Set the computer's system-wide DNS servers
    ///
    /// The `mut` in `&mut self` is not needed by Rust's rules, but
    /// it would be bad if this was called from 2 threads at once.
    ///
    /// Must be async to match the Linux signature
    #[allow(clippy::unused_async)]
    pub(crate) async fn set_dns(&mut self, dns_config: &[IpAddr]) -> Result<()> {
        activate(dns_config).context("Failed to activate DNS control")?;
        Ok(())
    }

    /// Flush Windows' system-wide DNS cache
    ///
    /// `&self` is needed to match the Linux signature
    pub(crate) fn flush(&self) -> Result<()> {
        tracing::debug!("Flushing Windows DNS cache...");
        Command::new("ipconfig")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/flushdns"])
            .status()?;
        tracing::debug!("Flushed DNS.");
        Ok(())
    }
}

pub(crate) fn system_resolvers() -> Result<Vec<IpAddr>> {
    let resolvers = ipconfig::get_adapters()?
        .iter()
        .flat_map(|adapter| adapter.dns_servers())
        .filter(|ip| match ip {
            IpAddr::V4(_) => true,
            // Filter out bogus DNS resolvers on my dev laptop that start with fec0:
            IpAddr::V6(ip) => !ip.octets().starts_with(&[0xfe, 0xc0]),
        })
        .copied()
        .collect();
    // This is private, so keep it at `debug` or `trace`
    tracing::debug!(?resolvers);
    Ok(resolvers)
}

/// A UUID for the Firezone Client NRPT rule, chosen randomly at dev time.
///
/// Our NRPT rule should always live in the registry at
/// `Computer\HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\Dnscache\Parameters\DnsPolicyConfig\$NRPT_REG_KEY`
///
/// We can use this UUID as a handle to enable, disable, or modify the rule.
const NRPT_REG_KEY: &str = "{6C0507CB-C884-4A78-BC55-0ACEE21227F6}";

/// Tells Windows to send all DNS queries to our sentinels
///
/// Parameters:
/// - `dns_config_string`: Comma-separated IP addresses of DNS servers, e.g. "1.1.1.1,8.8.8.8"
fn activate(dns_config: &[IpAddr]) -> Result<()> {
    // TODO: Known issue where web browsers will keep a connection open to a site,
    // using QUIC, HTTP/2, or even HTTP/1.1, and so they won't resolve the DNS
    // again unless you let that connection time out:
    // <https://github.com/firezone/firezone/issues/3113#issuecomment-1882096111>

    tracing::info!("Activating DNS control");
    let dns_config_string = dns_config
        .iter()
        .map(|ip| ip.to_string())
        .collect::<Vec<_>>()
        .join(";");

    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let base = Path::new("SYSTEM")
        .join("CurrentControlSet")
        .join("Services")
        .join("Dnscache")
        .join("Parameters")
        .join("DnsPolicyConfig")
        .join(NRPT_REG_KEY);

    let (key, _) = hkcu.create_subkey(base)?;
    key.set_value("Comment", &FZ_MAGIC)?;
    key.set_value("ConfigOptions", &0x8u32)?;
    key.set_value("DisplayName", &"Firezone SplitDNS")?;
    key.set_value("GenericDNSServers", &dns_config_string)?;
    key.set_value("IPSECCARestriction", &"")?;
    key.set_value("Name", &vec!["."])?;
    key.set_value("Version", &0x2u32)?;

    Ok(())
}
