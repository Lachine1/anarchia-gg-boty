// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! DNS (SRV) resolution and SOCKS5 egress probing.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Same nameservers, TCP transport only.
fn tcp_only(cfg: ResolverConfig) -> ResolverConfig {
    use hickory_resolver::config::{NameServerConfig, Protocol};
    let mut out = ResolverConfig::from_parts(cfg.domain().cloned(), cfg.search().to_vec(), vec![]);
    let mut seen = std::collections::HashSet::new();
    for ns in cfg.name_servers() {
        if seen.insert(ns.socket_addr) {
            out.add_name_server(NameServerConfig::new(ns.socket_addr, Protocol::Tcp));
        }
    }
    out
}

fn cloudflare() -> TokioAsyncResolver {
    TokioAsyncResolver::tokio(tcp_only(ResolverConfig::cloudflare()), ResolverOpts::default())
}

fn resolver() -> TokioAsyncResolver {
    // system DNS (falls back to Cloudflare on Termux/Android where there is no resolv.conf)
    match hickory_resolver::system_conf::read_system_conf() {
        Ok((cfg, opts)) if !cfg.name_servers().is_empty() => TokioAsyncResolver::tokio(tcp_only(cfg), opts),
        _ => cloudflare(),
    }
}

/// Resolve a Minecraft server address like vanilla: `_minecraft._tcp` SRV, then A record.
pub async fn resolve(server: &str) -> Result<SocketAddr> {
    let (host, port) = match server.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() => (h.to_string(), Some(p.parse::<u16>()?)),
        _ => (server.to_string(), None),
    };
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, port.unwrap_or(25565)));
    }
    let mut r = resolver();
    let (mut target, mut port) = (host.clone(), port.unwrap_or(25565));
    if server.rsplit_once(':').map_or(true, |(_, p)| p.parse::<u16>().is_err()) {
        let srv = match r.srv_lookup(format!("_minecraft._tcp.{host}.")).await {
            Ok(s) => Ok(s),
            Err(_) => {
                r = cloudflare();
                r.srv_lookup(format!("_minecraft._tcp.{host}.")).await
            }
        };
        if let Ok(srv) = srv {
            if let Some(rec) = srv.iter().next() {
                target = rec.target().to_utf8().trim_end_matches('.').to_string();
                port = rec.port();
            }
        }
    }
    let ips = match r.ipv4_lookup(format!("{target}.")).await {
        Ok(v) => v,
        Err(_) => cloudflare()
            .ipv4_lookup(format!("{target}."))
            .await
            .with_context(|| format!("brak rekordu A dla {target}"))?,
    };
    let ip = ips.iter().next().ok_or_else(|| anyhow!("brak rekordu A dla {target}"))?;
    Ok(SocketAddr::new(std::net::IpAddr::V4(ip.0), port))
}

/// Public IPv4 seen by the world through a SOCKS5 proxy (Cloudflare trace over plain HTTP).
pub async fn egress_ip(proxy: SocketAddr, auth: Option<(String, String)>) -> Result<String> {
    let fut = async {
        let mut s = TcpStream::connect(proxy).await?;
        let key = auth.map(|(u, p)| socks5_impl::protocol::UserKey::new(u, p));
        // cloudflare.com resolves to Cloudflare edge; Host header picks the trace endpoint
        socks5_impl::client::connect(&mut s, ("104.16.132.229".to_string(), 80u16), key).await.map_err(|e| anyhow!("socks: {e}"))?;
        s.write_all(b"GET /cdn-cgi/trace HTTP/1.1\r\nHost: cloudflare.com\r\nUser-Agent: curl/8\r\nConnection: close\r\n\r\n").await?;
        let mut buf = Vec::with_capacity(1024);
        s.read_to_end(&mut buf).await?;
        let t = String::from_utf8_lossy(&buf);
        t.lines().find_map(|l| l.strip_prefix("ip=")).map(|s| s.trim().to_string()).ok_or_else(|| anyhow!("brak ip w odpowiedzi"))
    };
    tokio::time::timeout(Duration::from_secs(10), fut).await.map_err(|_| anyhow!("timeout"))?
}
