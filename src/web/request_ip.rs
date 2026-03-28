// request_ip.rs
// Copyright 2026 Patrick Meade.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![allow(dead_code)]

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, header, request::Parts},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestIp(pub Option<IpAddr>);

impl RequestIp {
    pub fn get(self) -> Option<IpAddr> {
        self.0
    }
}

impl<S> FromRequestParts<S> for RequestIp
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ip = forwarded_ip(&parts.headers).or_else(|| x_forwarded_for_ip(&parts.headers));
        let ip = match ip {
            Some(ip) => Some(ip),
            None => ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
                .await
                .ok()
                .map(|connect_info| connect_info.0.ip()),
        };

        Ok(Self(ip))
    }
}

fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    let value = headers.get(header::FORWARDED)?.to_str().ok()?;

    value.split(',').find_map(|entry| {
        entry.split(';').find_map(|part| {
            let (name, value) = part.split_once('=')?;
            if name.trim().eq_ignore_ascii_case("for") {
                parse_ip(value)
            } else {
                None
            }
        })
    })
}

fn x_forwarded_for_ip(headers: &HeaderMap) -> Option<IpAddr> {
    let value = headers.get("x-forwarded-for")?.to_str().ok()?;

    value.split(',').find_map(parse_ip)
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"');

    if value.is_empty() || value.eq_ignore_ascii_case("unknown") {
        return None;
    }

    if let Some(value) = value.strip_prefix('[') {
        let (ip, _) = value.split_once(']')?;
        return IpAddr::from_str(ip).ok();
    }

    IpAddr::from_str(value)
        .ok()
        .or_else(|| SocketAddr::from_str(value).ok().map(|addr| addr.ip()))
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::{forwarded_ip, parse_ip, x_forwarded_for_ip};

    #[test]
    fn parses_forwarded_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::FORWARDED,
            HeaderValue::from_static("for=203.0.113.10;proto=https"),
        );

        assert_eq!(
            forwarded_ip(&headers).map(|ip| ip.to_string()),
            Some("203.0.113.10".to_string())
        );
    }

    #[test]
    fn parses_forwarded_header_with_ipv6_and_port() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::FORWARDED,
            HeaderValue::from_static("for=\"[2001:db8::1]:8443\""),
        );

        assert_eq!(
            forwarded_ip(&headers).map(|ip| ip.to_string()),
            Some("2001:db8::1".to_string())
        );
    }

    #[test]
    fn parses_x_forwarded_for_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.10, 198.51.100.11"),
        );

        assert_eq!(
            x_forwarded_for_ip(&headers).map(|ip| ip.to_string()),
            Some("198.51.100.10".to_string())
        );
    }

    #[test]
    fn parse_ip_rejects_unknown_and_invalid_values() {
        assert!(parse_ip("unknown").is_none());
        assert!(parse_ip("not-an-ip").is_none());
    }
}
