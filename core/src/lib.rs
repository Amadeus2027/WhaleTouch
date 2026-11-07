//! Hardware-independent protocol and interaction rules.
//! USB report offsets and vertical mapping adapted from DukeCCY/dualsense-pad (MIT).
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub active: bool,
    pub x: u16,
    pub y: u16,
}

pub fn parse_usb(data: &[u8]) -> Result<Point, &'static str> {
    if data.len() != 64 || data[0] != 0x01 {
        return Err("仅支持 64 字节 USB 0x01 报文");
    }
    let point = |base: usize| Point {
        active: data[base] & 0x80 == 0,
        x: data[base + 1] as u16 | ((data[base + 2] as u16 & 15) << 8),
        y: (data[base + 2] as u16 >> 4) | ((data[base + 3] as u16) << 4),
    };
    let a = point(33);
    let b = point(37);
    let p = if a.active { a } else { b };
    if p.active && (p.x >= 1920 || p.y >= 1080) {
        return Err("触点超出范围，请确认 USB 连接");
    }
    Ok(p)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Head,
    Neck,
    Chest,
    Waist,
    Hips,
    Thigh,
    Shin,
    Foot,
}
impl Zone {
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Head => [72, 126, 210],
            Self::Neck => [99, 151, 203],
            Self::Chest => [122, 139, 205],
            Self::Waist => [109, 162, 191],
            Self::Hips => [145, 142, 195],
            Self::Thigh => [110, 158, 181],
            Self::Shin => [90, 168, 187],
            Self::Foot => [79, 174, 166],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mapping {
    pub chibi: bool,
    pub flip_u: bool,
    pub flip_v: bool,
}
impl Mapping {
    // Physical front-view LEFT = head, RIGHT = feet. Do not rotate the raw axes twice.
    pub fn map(self, p: Point) -> (f32, f32, Zone) {
        let mut u = 1.0 - p.y as f32 / 1080.0;
        let mut v = p.x as f32 / 1920.0;
        if self.flip_u {
            u = 1.0 - u;
        }
        if self.flip_v {
            v = 1.0 - v;
        }
        (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0), self.zone(v))
    }
    pub fn zone(self, v: f32) -> Zone {
        let ends = if self.chibi {
            [0.35, 0.40, 0.52, 0.60, 0.70, 0.84, 0.94]
        } else {
            [0.18, 0.26, 0.40, 0.52, 0.62, 0.78, 0.92]
        };
        let i = ends.iter().position(|end| v < *end).unwrap_or(7);
        [
            Zone::Head,
            Zone::Neck,
            Zone::Chest,
            Zone::Waist,
            Zone::Hips,
            Zone::Thigh,
            Zone::Shin,
            Zone::Foot,
        ][i]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TouchFrame {
    pub active: bool,
    pub u: f32,
    pub v: f32,
    pub zone: Zone,
    pub speed: f32,
    pub gesture: String,
    pub ended: bool,
}
pub struct TouchTracker {
    samples: VecDeque<(u64, Point)>,
    start: Option<u64>,
    path: f32,
    last: Option<Point>,
    frame: TouchFrame,
}
impl Default for TouchTracker {
    fn default() -> Self {
        Self {
            samples: VecDeque::new(),
            start: None,
            path: 0.0,
            last: None,
            frame: TouchFrame {
                active: false,
                u: 0.5,
                v: 0.0,
                zone: Zone::Head,
                speed: 0.0,
                gesture: String::new(),
                ended: false,
            },
        }
    }
}
impl TouchTracker {
    pub fn update(&mut self, p: Point, now: u64, mapping: Mapping) -> TouchFrame {
        self.frame.ended = false;
        if p.active {
            if self.start.is_none() {
                self.start = Some(now);
                self.path = 0.0;
                self.samples.clear();
            }
            if let Some(last) = self.last {
                self.path += distance(p, last);
            }
            self.samples.push_back((now, p));
            while self.samples.len() > 1 && now.saturating_sub(self.samples[0].0) > 120 {
                self.samples.pop_front();
            }
            let (at, first) = self.samples[0];
            let speed = if now > at {
                distance(p, first) * 1000.0 / (now - at) as f32
            } else {
                0.0
            };
            let (u, v, zone) = mapping.map(p);
            self.frame = TouchFrame {
                active: true,
                u,
                v,
                zone,
                speed,
                gesture: if speed > 120.0 {
                    "rub"
                } else if self.path > 4.0 {
                    "stroke"
                } else {
                    "hold"
                }
                .into(),
                ended: false,
            };
            self.last = Some(p);
        } else if let Some(start) = self.start.take() {
            self.frame.active = false;
            self.frame.speed = 0.0;
            self.frame.ended = true;
            self.frame.gesture = if now.saturating_sub(start) < 300 && self.path < 4.0 {
                "tap"
            } else if self.path > 4.0 {
                "stroke"
            } else {
                "hold"
            }
            .into();
            self.last = None;
            self.samples.clear();
        }
        self.frame.clone()
    }
}
fn distance(a: Point, b: Point) -> f32 {
    (((a.x as f32 - b.x as f32) * 40.0 / 1920.0).powi(2)
        + ((a.y as f32 - b.y as f32) * 25.0 / 1080.0).powi(2))
    .sqrt()
}

/// One writer owns both motors and LED. Output values never wrap around.
pub fn output_report(weak: u8, strong: u8, rgb: [u8; 3], release: bool) -> [u8; 48] {
    let mut out = [0; 48];
    out[0] = 0x02;
    out[1] = 0x03;
    out[2] = if release { 0x08 } else { 0x04 };
    if !release {
        out[3] = weak;
        out[4] = strong;
        out[45..48].copy_from_slice(&rgb);
    }
    out
}
pub fn feedback(frame: &TouchFrame, gain: f32, now_ms: u64) -> (u8, u8) {
    if !frame.active || gain <= 0.0 {
        return (0, 0);
    }
    let speed = (frame.speed / 160.0).clamp(0.0, 1.0);
    let duty = 0.18 + 0.62 * speed;
    if (now_ms % 100) as f32 >= duty * 100.0 {
        return (0, 0);
    }
    let g = gain.clamp(0.0, 1.0);
    (
        ((20.0 + 45.0 * speed) * g) as u8,
        ((8.0 + 18.0 * speed) * g) as u8,
    )
}

/// SSE decoding tolerates chunk boundaries, UTF-8 splits and CRLF.
#[derive(Default)]
pub struct SseDecoder {
    pending: Vec<u8>,
}
impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 1024 * 1024 {
            return Err("流式响应片段过大".into());
        }
        let mut result = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let s = std::str::from_utf8(&line)
                .map_err(|_| "无效 UTF-8")?
                .trim_end_matches(['\r', '\n']);
            if let Some(data) = s.strip_prefix("data:") {
                result.push(data.trim_start().to_string());
            }
        }
        Ok(result)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub currency: String,
    pub total_balance: String,
    pub granted_balance: String,
    pub topped_up_balance: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Balance {
    pub is_available: bool,
    pub balance_infos: Vec<BalanceInfo>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub companion_seconds: u64,
    pub feeds: u64,
    pub usage: Usage,
    pub usage_by_day: std::collections::BTreeMap<String, Usage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usb_and_vertical_regions() {
        let mut r = [0u8; 64];
        r[0] = 1;
        r[33] = 0;
        r[34] = 0x7f;
        r[35] = 0x77;
        r[36] = 0x43;
        r[37] = 0x80;
        let p = parse_usb(&r).unwrap();
        assert_eq!((p.x, p.y), (1919, 1079));
        assert!(parse_usb(&r[..40]).is_err());
        r[0] = 0x31;
        assert!(parse_usb(&r).is_err());
    }
    #[test]
    fn mapping_boundaries_and_chibi() {
        let m = Mapping::default();
        assert_eq!(
            m.map(Point {
                active: true,
                x: 0,
                y: 0
            })
            .2,
            Zone::Head
        );
        assert_eq!(
            m.map(Point {
                active: true,
                x: 1919,
                y: 1079
            })
            .2,
            Zone::Foot
        );
        assert_eq!(m.zone(0.18), Zone::Neck);
        assert_eq!(m.zone(0.92), Zone::Foot);
        assert_eq!(Mapping { chibi: true, ..m }.zone(0.30), Zone::Head);
        assert!(Mapping { flip_v: true, ..m }.map(Point::default()).1 > 0.99);
    }
    #[test]
    fn touch_start_end_and_haptics_stop() {
        let mut t = TouchTracker::default();
        let p = Point {
            active: true,
            x: 100,
            y: 200,
        };
        assert!(t.update(p, 0, Mapping::default()).active);
        let ended = t.update(Point::default(), 100, Mapping::default());
        assert!(ended.ended);
        assert_eq!(ended.gesture, "tap");
        assert_eq!(feedback(&ended, 1.0, 100), (0, 0));
        assert!(!t.update(Point::default(), 200, Mapping::default()).ended);
        let out = output_report(20, 8, [1, 2, 3], true);
        assert_eq!((out[2], out[3], out[4]), (8, 0, 0));
    }
    #[test]
    fn sse_split_and_balance() {
        let mut s = SseDecoder::default();
        assert!(s.push(b"data: {\"choices\":").unwrap().is_empty());
        let rows = s.push(b"[]}\r\n\r\ndata: [DONE]\n\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], "[DONE]");
        let b:Balance=serde_json::from_str(r#"{"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"12.30","granted_balance":"2.00","topped_up_balance":"10.30"}]}"#).unwrap();
        assert_eq!(b.balance_infos[0].total_balance, "12.30");
    }
}
