// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;

lazy_static! {
    static ref DURATION_RE: Regex = Regex::new(
        r"(?x)
^
((?P<y>[0-9]+)y)?
((?P<w>[0-9]+)w)?
((?P<d>[0-9]+)d)?
((?P<h>[0-9]+)h)?
((?P<m>[0-9]+)m)?
((?P<s>[0-9]+)s)?
((?P<ms>[0-9]+)ms)?
$",
    )
    .unwrap();
}

pub const MILLI_DURATION: Duration = Duration::from_millis(1);
pub const SECOND_DURATION: Duration = Duration::from_secs(1);
pub const MINUTE_DURATION: Duration = Duration::from_secs(60);
pub const HOUR_DURATION: Duration = Duration::from_secs(60 * 60);
pub const DAY_DURATION: Duration = Duration::from_secs(60 * 60 * 24);
pub const WEEK_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);
pub const YEAR_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 365);

const ALL_CAPS: [(&str, Duration); 7] = [
    ("y", YEAR_DURATION),
    ("w", WEEK_DURATION),
    ("d", DAY_DURATION),
    ("h", HOUR_DURATION),
    ("m", MINUTE_DURATION),
    ("s", SECOND_DURATION),
    ("ms", MILLI_DURATION),
];

/// parses a string into a Duration, assuming that a year
/// always has 365d, a week always has 7d, and a day always has 24h.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::time::Duration;
/// use promql_parser::util;
///
/// assert_eq!(util::parse_duration("1h").unwrap(), Duration::from_secs(3600));
/// assert_eq!(util::parse_duration("4d").unwrap(), Duration::from_secs(3600 * 24 * 4));
/// assert_eq!(util::parse_duration("4d1h").unwrap(), Duration::from_secs(3600 * 97));
/// ```
pub fn parse_duration(ds: &str) -> Result<Duration, String> {
    if ds.is_empty() {
        return Err("empty duration string".into());
    }

    if ds == "0" {
        return Err("duration must be greater than 0".into());
    }

    if !DURATION_RE.is_match(ds) {
        return Err(format!("not a valid duration string: {ds}"));
    }

    let caps = DURATION_RE.captures(ds).unwrap();
    let dur = ALL_CAPS
        .into_iter()
        // map captured string to Option<Duration> iterator
        // FIXME: None is ignored in closure. It is better to tell users which part is wrong.
        .map(|(title, duration)| {
            caps.name(title)
                .and_then(|cap| cap.as_str().parse::<u32>().ok())
                .and_then(|v| duration.checked_mul(v))
        })
        .fold(Ok(Duration::ZERO), |acc, x| {
            acc.and_then(|d| {
                d.checked_add(x.unwrap_or(Duration::ZERO))
                    .ok_or_else(|| "duration overflowed".into())
            })
        });

    if matches!(dur, Ok(d) if d == Duration::ZERO) {
        Err("duration must be greater than 0".into())
    } else {
        dur
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re() {
        // valid regex
        let res = vec![
            "1y", "2w", "3d", "4h", "5m", "6s", "7ms", "1y2w3d", "4h30m", "3600ms",
        ];
        for re in res {
            assert!(DURATION_RE.is_match(re), "{} failed.", re)
        }

        // invalid regex
        let res = vec!["1", "1y1m1d", "-1w", "1.5d", "d"];
        for re in res {
            assert!(!DURATION_RE.is_match(re), "{} failed.", re)
        }
    }

    #[test]
    fn test_valid_duration() {
        let ds = vec![
            ("324ms", Duration::from_millis(324)),
            ("3s", Duration::from_secs(3)),
            ("5m", MINUTE_DURATION * 5),
            ("1h", HOUR_DURATION),
            ("4d", DAY_DURATION * 4),
            ("4d1h", DAY_DURATION * 4 + HOUR_DURATION),
            ("14d", DAY_DURATION * 14),
            ("3w", WEEK_DURATION * 3),
            ("3w2d1h", WEEK_DURATION * 3 + HOUR_DURATION * 49),
            ("10y", YEAR_DURATION * 10),
        ];

        for (s, expect) in ds {
            let d = parse_duration(s);
            assert!(d.is_ok());
            assert_eq!(expect, d.unwrap(), "{} and {:?} not matched", s, expect);
        }
    }

    // valid here but invalid in PromQL Go Version
    #[test]
    fn test_diff_with_promql() {
        let ds = vec![
            ("294y", YEAR_DURATION * 294),
            ("200y10400w", YEAR_DURATION * 200 + WEEK_DURATION * 10400),
            ("107675d", DAY_DURATION * 107675),
            ("2584200h", HOUR_DURATION * 2584200),
        ];

        for (s, expect) in ds {
            let d = parse_duration(s);
            assert!(d.is_ok());
            assert_eq!(expect, d.unwrap(), "{} and {:?} not matched", s, expect);
        }
    }

    #[test]
    fn test_invalid_duration() {
        let ds = vec!["1", "1y1m1d", "-1w", "1.5d", "d", "", "0", "0w", "0s"];
        for d in ds {
            assert!(parse_duration(d).is_err(), "{} is invalid duration!", d);
        }
    }
}
