use chrono::{Datelike, Timelike};
use std::time::Instant;

const RTC_STOP: u16 = 0x0004;
const RTC_STOPPED_STATUS: u8 = 0x80;
const RTC_BLOCK_BYTES: usize = 8;

#[derive(Clone)]
pub struct AfRtc {
    pub control: u16,
    calibration: [u8; 6],
    user: [u8; RTC_BLOCK_BYTES],
    time: chrono::NaiveDateTime,
    epoch: Instant,
}

impl Default for AfRtc {
    fn default() -> Self {
        Self {
            control: 0,
            calibration: [0; 6],
            user: [0; RTC_BLOCK_BYTES],
            time: chrono::Local::now().naive_local(),
            epoch: Instant::now(),
        }
    }
}

fn bcd(value: u32) -> u8 {
    let value = value % 100;
    (((value / 10) << 4) | (value % 10)) as u8
}

fn decode_bcd(value: u8) -> Option<u32> {
    let tens = value >> 4;
    let ones = value & 0xf;
    (tens <= 9 && ones <= 9).then_some(tens as u32 * 10 + ones as u32)
}

impl AfRtc {
    pub(crate) fn status(&self) -> u8 {
        if self.control & RTC_STOP != 0 {
            RTC_STOPPED_STATUS
        } else {
            0
        }
    }

    fn current_time(&self) -> chrono::NaiveDateTime {
        if self.status() != 0 {
            self.time
        } else {
            self.time
                .checked_add_signed(chrono::TimeDelta::seconds(
                    self.epoch.elapsed().as_secs() as i64
                ))
                .unwrap_or(self.time)
        }
    }

    pub(crate) fn read(&self, block: u8) -> Option<[u8; RTC_BLOCK_BYTES]> {
        match block {
            0 => {
                let mut data = [0; RTC_BLOCK_BYTES];
                data[..2].copy_from_slice(&self.control.to_be_bytes());
                data[2..].copy_from_slice(&self.calibration);
                Some(data)
            }
            1 => Some(self.user),
            2 => {
                let now = self.current_time();
                let year = now.year().saturating_sub(1900) as u32;
                Some([
                    bcd(now.second()),
                    bcd(now.minute()),
                    0x80 | bcd(now.hour()),
                    bcd(now.day()),
                    bcd(now.weekday().num_days_from_sunday()),
                    bcd(now.month()),
                    bcd(year),
                    bcd(year / 100),
                ])
            }
            _ => None,
        }
    }

    pub(crate) fn write(&mut self, block: u8, data: [u8; RTC_BLOCK_BYTES]) -> bool {
        match block {
            0 => {
                self.time = self.current_time();
                self.epoch = Instant::now();
                self.control = u16::from_be_bytes([data[0], data[1]]);
                self.calibration.copy_from_slice(&data[2..]);
            }
            1 if self.control & 0x0100 == 0 => self.user = data,
            2 if self.control & 0x0200 == 0 => {
                let Some(time) = decode_time(data) else {
                    return false;
                };
                self.time = time;
                self.epoch = Instant::now();
            }
            1 | 2 => {}
            _ => return false,
        }
        true
    }
}

fn decode_time(data: [u8; RTC_BLOCK_BYTES]) -> Option<chrono::NaiveDateTime> {
    let year = 1900 + decode_bcd(data[6])? + decode_bcd(data[7])? * 100;
    chrono::NaiveDate::from_ymd_opt(year as i32, decode_bcd(data[5])?, decode_bcd(data[3])?)?
        .and_hms_opt(
            decode_bcd(data[2] & 0x7f)?,
            decode_bcd(data[1])?,
            decode_bcd(data[0])?,
        )
}
