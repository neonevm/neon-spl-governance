use chrono::{Duration, NaiveDateTime};
use chronoutil::delta::{shift_months, shift_years};
pub use spl_governance_addin_vesting::state::VestingSchedule;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Lockup {
    NoLockup,
    For4Years,
    For1year1yearLinear,
}

impl Lockup {
    pub fn default() -> Self {
        Lockup::For1year1yearLinear
    }

    pub fn is_locked(&self) -> bool {
        *self != Lockup::NoLockup
    }

    pub fn get_schedule_size(&self) -> u32 {
        match *self {
            Lockup::NoLockup => 1,
            Lockup::For4Years => 1,
            Lockup::For1year1yearLinear => 12,
        }
    }

    pub fn get_mainnet_schedule(
        &self,
        start_time: NaiveDateTime,
        amount: u64,
    ) -> Vec<VestingSchedule> {
        match self {
            Lockup::NoLockup => {
                vec![VestingSchedule {
                    release_time: 0,
                    amount,
                }]
            }
            Lockup::For4Years => {
                vec![VestingSchedule {
                    release_time: shift_years(start_time, 4).timestamp() as u64,
                    amount,
                }]
            }
            Lockup::For1year1yearLinear => {
                let mut schedules = vec![];
                let start = shift_years(start_time, 1);
                for i in 1i32..=12 {
                    let prev = (amount as u128)
                        .checked_mul((i - 1) as u128)
                        .unwrap()
                        .checked_div(12)
                        .unwrap() as u64;
                    let curr = (amount as u128)
                        .checked_mul(i as u128)
                        .unwrap()
                        .checked_div(12)
                        .unwrap() as u64;
                    let schedule = VestingSchedule {
                        release_time: shift_months(start, i).timestamp() as u64,
                        amount: curr - prev,
                    };
                    schedules.push(schedule)
                }
                schedules
            }
        }
    }

    pub fn get_testing_schedule(
        &self,
        start_time: NaiveDateTime,
        amount: u64,
    ) -> Vec<VestingSchedule> {
        match self {
            Lockup::NoLockup => {
                vec![VestingSchedule {
                    release_time: 0,
                    amount,
                }]
            }
            Lockup::For4Years => {
                vec![VestingSchedule {
                    release_time: (start_time + Duration::minutes(4 * 12)).timestamp() as u64,
                    amount,
                }]
            }
            Lockup::For1year1yearLinear => {
                let mut schedules = vec![];
                let start = start_time + Duration::minutes(12);
                for i in 1i32..=12 {
                    let prev = (amount as u128)
                        .checked_mul((i - 1) as u128)
                        .unwrap()
                        .checked_div(12)
                        .unwrap() as u64;
                    let curr = (amount as u128)
                        .checked_mul(i as u128)
                        .unwrap()
                        .checked_div(12)
                        .unwrap() as u64;
                    let schedule = VestingSchedule {
                        release_time: (start + Duration::seconds((i * 12 * 60 / 12).into()))
                            .timestamp() as u64,
                        amount: curr - prev,
                    };
                    schedules.push(schedule)
                }
                schedules
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::info;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_date() {
        let today = Utc::today();
        let _curr = today.naive_utc().and_hms(0, 0, 0);
        let curr = Utc.ymd(2022, 1, 31).and_hms(0, 0, 0);

        info!("Today: {}", today);
        info!("Current:    {} {}", curr, curr.timestamp());

        for i in 1..=12 {
            let curr2 = shift_months(curr, i);
            info!("Current {:2}: {} {}", i, curr2, curr2.timestamp());
        }
    }
}
