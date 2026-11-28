//! Reading a point from what the operator copies in Google Maps: a shared
//! link, the address of the browser or the coordinates shown for a place.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaceInput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceView {
    pub latitude: f64,
    pub longitude: f64,
    /// The name the link carries, when it is more than the coordinates.
    pub name: Option<String>,
}

impl PlaceView {
    /// What to call the place when the operator has not named it.
    #[must_use]
    pub fn title(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("{:.5}, {:.5}", self.latitude, self.longitude))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PastedPlace {
    Point(PlaceView),
    /// A shared link; the address behind it has to be asked for.
    ShortLink(String),
}

/// `None` when the text names no point.
#[must_use]
pub fn read_pasted_place(text: &str) -> Option<PastedPlace> {
    // A shared message may carry the name of the place before the link.
    let link = text
        .split_whitespace()
        .find(|word| word.starts_with("https://") || word.starts_with("http://"));
    match link {
        Some(link) => read_link(link),
        None => coordinates(text).map(|point| found(point, None)),
    }
}

fn found((latitude, longitude): (f64, f64), name: Option<String>) -> PastedPlace {
    let rounded = |value: f64| (value * 1e6).round() / 1e6;
    PastedPlace::Point(PlaceView {
        latitude: rounded(latitude),
        longitude: rounded(longitude),
        name,
    })
}

fn read_link(link: &str) -> Option<PastedPlace> {
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))?;
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (host, path) = rest.split_at(host_end);
    if host == "maps.app.goo.gl" || (host == "goo.gl" && path.starts_with("/maps/")) {
        return Some(PastedPlace::ShortLink(format!("https://{rest}")));
    }
    let google = ["www.google.", "google.", "maps.google.", "consent.google."]
        .iter()
        .any(|start| host.starts_with(start));
    if !google {
        return None;
    }

    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let segments: Vec<String> = route.split('/').map(decoded_words).collect();
    let after = |word: &str| {
        segments
            .iter()
            .position(|segment| segment == word)
            .and_then(|at| segments.get(at + 1))
    };
    let name = after("place")
        .filter(|segment| !segment.is_empty() && coordinates(segment).is_none())
        .filter(|segment| !segment.starts_with('@') && !segment.starts_with("data="))
        .cloned();

    // The pin of the place. A consent page carries the whole address encoded.
    let whole = decoded(&decoded(path));
    if let Some(point) = pin(&whole) {
        return Some(found(point, name));
    }
    for word in ["place", "search"] {
        if let Some(point) = after(word).and_then(|segment| coordinates(segment)) {
            return Some(found(point, None));
        }
    }
    for part in query.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        if ["q", "query", "destination", "ll"].contains(&key)
            && let Some(point) = coordinates(&decoded_words(value))
        {
            return Some(found(point, None));
        }
    }
    // The centre of the shown map (`/@lat,lon`) is not a chosen point.
    None
}

/// `%XX` sequences replaced by the bytes they stand for.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = (bytes[at] == b'%')
            .then(|| bytes.get(at + 1..at + 3))
            .flatten()
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match hex {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A part of an address where `+` stands for a space.
fn decoded_words(text: &str) -> String {
    decoded(&text.replace('+', " "))
}

fn leading_number(text: &str) -> Option<(f64, &str)> {
    let end = text
        .char_indices()
        .find(|&(at, ch)| !(ch.is_ascii_digit() || ch == '.' || (at == 0 && ch == '-')))
        .map_or(text.len(), |(at, _)| at);
    Some((text[..end].parse().ok()?, &text[end..]))
}

/// The last `!3d<latitude>!4d<longitude>` of the address.
fn pin(address: &str) -> Option<(f64, f64)> {
    address.rmatch_indices("!3d").find_map(|(at, mark)| {
        let (latitude, rest) = leading_number(&address[at + mark.len()..])?;
        let (longitude, _) = leading_number(rest.strip_prefix("!4d")?)?;
        in_range(latitude, longitude)
    })
}

fn in_range(latitude: f64, longitude: f64) -> Option<(f64, f64)> {
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude))
        .then_some((latitude, longitude))
}

fn coordinates(text: &str) -> Option<(f64, f64)> {
    degrees_pair(text).or_else(|| decimal_pair(text))
}

/// `49.5, 24.5`, with or without brackets. Both numbers need a fraction.
fn decimal_pair(text: &str) -> Option<(f64, f64)> {
    let plain = text.trim().trim_start_matches('(').trim_end_matches(')');
    let parts: Vec<&str> = plain
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';')
        .filter(|part| !part.is_empty())
        .collect();
    let [first, second] = parts[..] else {
        return None;
    };
    if !(first.contains('.') && second.contains('.')) {
        return None;
    }
    in_range(first.parse().ok()?, second.parse().ok()?)
}

/// One side of `49°30'00.0"N 24°30'00.0"E`: its value, its letter and what follows.
fn degrees_part(text: &str) -> Option<(f64, char, &str)> {
    let letter = |text: &str| {
        text.chars()
            .next()
            .filter(|ch| matches!(ch, 'N' | 'S' | 'E' | 'W'))
    };
    let mut rest = text.trim_start_matches([' ', ',', ';']);
    let before = letter(rest);
    if before.is_some() {
        rest = rest[1..].trim_start();
    }
    let (mut value, after) = leading_number(rest)?;
    rest = after.trim_start().strip_prefix('°')?;
    if !(0.0..=180.0).contains(&value) {
        return None;
    }
    for (mark, size) in [('\'', 60.0), ('"', 3600.0)] {
        if let Some((part, after)) = leading_number(rest.trim_start())
            && let Some(after) = after.trim_start().strip_prefix(mark)
        {
            if !(0.0..60.0).contains(&part) {
                return None;
            }
            value += part / size;
            rest = after;
        }
    }
    if let Some(letter) = before {
        return Some((value, letter, rest));
    }
    rest = rest.trim_start();
    let found = letter(rest)?;
    Some((value, found, &rest[1..]))
}

/// Degrees, minutes and seconds as Google Maps writes them.
fn degrees_pair(text: &str) -> Option<(f64, f64)> {
    let plain = text
        .replace(['′', '’'], "'")
        .replace(['″', '”'], "\"")
        .replace("''", "\"");
    let (first, first_letter, rest) = degrees_part(plain.trim())?;
    let (second, second_letter, rest) = degrees_part(rest)?;
    if !rest.trim().is_empty() {
        return None;
    }
    let signed = |value: f64, letter: char| {
        if matches!(letter, 'S' | 'W') {
            -value
        } else {
            value
        }
    };
    let along = |letter: char| matches!(letter, 'N' | 'S');
    let first = (signed(first, first_letter), along(first_letter));
    let second = (signed(second, second_letter), along(second_letter));
    match (first, second) {
        ((latitude, true), (longitude, false)) | ((longitude, false), (latitude, true)) => {
            in_range(latitude, longitude)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(text: &str) -> Option<(f64, f64, Option<String>)> {
        match read_pasted_place(text)? {
            PastedPlace::Point(place) => Some((place.latitude, place.longitude, place.name)),
            PastedPlace::ShortLink(_) => None,
        }
    }

    #[test]
    fn coordinates_are_read_as_google_shows_them() {
        let half = Some((49.5, 24.5, None));
        assert_eq!(point("49.500000, 24.500000"), half);
        assert_eq!(point(" (49.5,24.5) "), half);
        assert_eq!(point("49°30'00.0\"N 24°30'00.0\"E"), half);
        assert_eq!(point("49°30′00″N, 24°30′00″E"), half);
        assert_eq!(point("N 49° 30' E 24° 30'"), half);
        assert_eq!(point("E24°30' N49°30'"), half);
        assert_eq!(
            point("33°30'00\"S 70°15'00\"W"),
            Some((-33.5, -70.25, None))
        );
        assert_eq!(
            point("49°30'36.0\"N 24°30'00\"E"),
            Some((49.51, 24.5, None))
        );
    }

    #[test]
    fn a_long_link_gives_the_pin_not_the_centre_of_the_map() {
        let link = "https://www.google.com/maps/place/49%C2%B030'00.0%22N+24%C2%B030'00.0%22E/\
                    @49.9,24.9,17z/data=!3m1!4b1!4m4!3m3!8m2!3d49.5!4d24.5?entry=tts";
        assert_eq!(point(link), Some((49.5, 24.5, None)));
        let named = "https://www.google.com/maps/place/%D0%A1%D0%BA%D0%BB%D0%B0%D0%B4+1/\
                     @49.9,24.9,17z/data=!4m6!3m5!1s0x0:0x0!8m2!3d49.5!4d24.5";
        assert_eq!(point(named), Some((49.5, 24.5, Some("Склад 1".to_owned()))));
        let consent = "https://consent.google.com/ml?continue=https://www.google.com/maps/place/X\
                       /data%3D%213d49.5%214d24.5&gl=DE";
        assert_eq!(point(consent), Some((49.5, 24.5, None)));
    }

    #[test]
    fn other_forms_of_a_link_are_read() {
        let half = Some((49.5, 24.5, None));
        for link in [
            "https://www.google.com/maps/search/49.5,+24.5?entry=tts",
            "https://www.google.com/maps/search/?api=1&query=49.5%2C24.5",
            "https://maps.google.com/?q=49.5,24.5",
            "https://www.google.com.ua/maps/place/49.5,24.5",
            "Місце: https://maps.google.com/maps?ll=49.5,24.5&z=15",
        ] {
            assert_eq!(point(link), half, "{link}");
        }
    }

    #[test]
    fn a_shared_link_is_asked_for_and_the_rest_is_refused() {
        for (text, link) in [
            (
                "https://maps.app.goo.gl/AbC123",
                "https://maps.app.goo.gl/AbC123",
            ),
            ("Склад\nhttp://goo.gl/maps/xyz", "https://goo.gl/maps/xyz"),
        ] {
            assert_eq!(
                read_pasted_place(text),
                Some(PastedPlace::ShortLink(link.to_owned()))
            );
        }
        for text in [
            "",
            "Склад 1",
            "12 34",
            "99.5, 24.5",
            "49°30'N 24°30'N",
            "https://www.google.com/maps/@49.5,24.5,17z",
            "https://example.org/maps?q=49.5,24.5",
            "https://goo.gl/abc",
        ] {
            assert_eq!(read_pasted_place(text), None, "{text}");
        }
    }
}
