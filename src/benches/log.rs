use alloc::vec::Vec;
use facet::Facet;
use rand::distr::StandardUniform;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

// Based on https://github.com/djkoloski/rust_serialization_benchmark/blob/master/src/datasets/log/mod.rs

#[derive(Debug, PartialEq, Facet, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
struct Address {
    x0: u8,
    x1: u8,
    x2: u8,
    x3: u8,
}

impl Distribution<Address> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Address {
        Address {
            x0: rng.random(),
            x1: rng.random(),
            x2: rng.random(),
            x3: rng.random(),
        }
    }
}

type StringPlaceholder = Vec<u8>; // TODO implement String.

fn string_placeholder(s: String) -> StringPlaceholder {
    s.into_bytes()
}

#[derive(Debug, PartialEq, Facet, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct LogEntry {
    address: Address,
    identity: StringPlaceholder,
    userid: StringPlaceholder,
    date: StringPlaceholder,
    request: StringPlaceholder,
    code: u16,
    size: u64,
}

impl Distribution<LogEntry> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LogEntry {
        const USERID: [&str; 9] = [
            "-", "alice", "bob", "carmen", "david", "eric", "frank", "george", "harry",
        ];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        const TIMEZONE: [&str; 25] = [
            "-1200", "-1100", "-1000", "-0900", "-0800", "-0700", "-0600", "-0500", "-0400",
            "-0300", "-0200", "-0100", "+0000", "+0100", "+0200", "+0300", "+0400", "+0500",
            "+0600", "+0700", "+0800", "+0900", "+1000", "+1100", "+1200",
        ];
        let date = format!(
            "{}/{}/{}:{}:{}:{} {}",
            rng.random_range(1..=28),
            MONTHS[rng.random_range(0..12)],
            rng.random_range(1970..=2021),
            rng.random_range(0..24),
            rng.random_range(0..60),
            rng.random_range(0..60),
            TIMEZONE[rng.random_range(0..25)],
        );
        const CODES: [u16; 63] = [
            100, 101, 102, 103, 200, 201, 202, 203, 204, 205, 206, 207, 208, 226, 300, 301, 302,
            303, 304, 305, 306, 307, 308, 400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410,
            411, 412, 413, 414, 415, 416, 417, 418, 421, 422, 423, 424, 425, 426, 428, 429, 431,
            451, 500, 501, 502, 503, 504, 505, 506, 507, 508, 510, 511,
        ];
        const METHODS: [&str; 5] = ["GET", "POST", "PUT", "UPDATE", "DELETE"];
        const ROUTES: [&str; 7] = [
            "/favicon.ico",
            "/css/index.css",
            "/css/font-awsome.min.css",
            "/img/logo-full.svg",
            "/img/splash.jpg",
            "/api/login",
            "/api/logout",
        ];
        const PROTOCOLS: [&str; 4] = ["HTTP/1.0", "HTTP/1.1", "HTTP/2", "HTTP/3"];
        let request = format!(
            "{} {} {}",
            METHODS[rng.random_range(0..5)],
            ROUTES[rng.random_range(0..7)],
            PROTOCOLS[rng.random_range(0..4)],
        );

        LogEntry {
            address: rng.sample(self),
            identity: string_placeholder("-".into()),
            userid: string_placeholder(USERID[rng.random_range(0..USERID.len())].into()),
            date: string_placeholder(date),
            request: string_placeholder(request),
            code: CODES[rng.random_range(0..CODES.len())],
            size: rng.random_range(0..100_000_000),
        }
    }
}

pub type Log = Vec<LogEntry>;

pub fn log(n: usize) -> Log {
    rand::rng().random_iter().take(n).collect()
}

pub fn log_one() -> Log {
    log(1)
}

pub fn log_1k() -> Log {
    log(1000)
}
