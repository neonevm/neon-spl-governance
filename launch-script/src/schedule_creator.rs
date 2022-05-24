use chrono::prelude::*;
use chronoutil::delta::{shift_months, shift_years};
use chrono::Duration;
use crate::Lockup;
use spl_governance_addin_vesting::state::VestingSchedule;

pub struct ScheduleCreator {
    pub current: NaiveDateTime,
    pub testing: bool,
}

impl ScheduleCreator {
    pub fn new(testing: bool) -> Self {
        let current = if testing {
            Utc::now().naive_utc()
        } else {
            Utc::today().naive_utc().and_hms(0, 0, 0)
        };
        Self {
            current,
            testing,
        }
    }

    pub fn get_schedule(&self, amount: u64, lockup: Lockup) -> Vec<VestingSchedule> {
        if self.testing {
            self._get_schedule_testing(amount, &lockup)
        } else {
            self._get_schedule_real(amount, lockup)
        }
    }

    fn _get_schedule_real(&self, amount: u64, lockup: Lockup) -> Vec<VestingSchedule> {
        match lockup {
            Lockup::NoLockup => {
                vec![
                    VestingSchedule {release_time: 0, amount}
                ]
            },
            Lockup::For4Years => {
                vec![
                    VestingSchedule {
                        release_time: shift_years(self.current, 4).timestamp() as u64,
                        amount
                    }
                ]
            },
            Lockup::For1Year_1YearLinear => {
                let mut schedules = vec!();
                let start = shift_years(self.current, 1);
                for i in 1i32..=12 {
                    let prev = (amount as u128)
                        .checked_mul((i-1) as u128).unwrap()
                        .checked_div(12).unwrap() as u64;
                    let curr = (amount as u128)
                        .checked_mul(i as u128).unwrap()
                        .checked_div(12).unwrap() as u64;
                    let schedule = VestingSchedule {
                        release_time: shift_months(start, i).timestamp() as u64,
                        amount: curr-prev
                    };
                    schedules.push(schedule)
                };
                schedules
            },
        }
    }

    fn _get_schedule_testing(&self, amount: u64, lockup: &Lockup) -> Vec<VestingSchedule> {
        match lockup {
            Lockup::NoLockup => {
                vec![
                    VestingSchedule {release_time: 0, amount}
                ]
            },
            Lockup::For4Years => {
                vec![
                    VestingSchedule {
                        release_time: (self.current + Duration::minutes(12)).timestamp() as u64,
                        amount
                    }
                ]
            },
            Lockup::For1Year_1YearLinear => {
                let mut schedules = vec!();
                let start = self.current + Duration::minutes(3);
                for i in 1i32..=12 {
                    let prev = (amount as u128)
                        .checked_mul((i-1) as u128).unwrap()
                        .checked_div(12).unwrap() as u64;
                    let curr = (amount as u128)
                        .checked_mul(i as u128).unwrap()
                        .checked_div(12).unwrap() as u64;
                    let schedule = VestingSchedule {
                        release_time: (start + Duration::seconds((i*180/12).into())).timestamp() as u64,
                        amount: curr-prev
                    };
                    schedules.push(schedule)
                };
                schedules
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_schedule_creator_testing() {
        let schedule_creator = ScheduleCreator::new(true);
        println!("{:?}", schedule_creator.get_schedule(11, Lockup::NoLockup));
        println!("{:?}", schedule_creator.get_schedule(11, Lockup::For4Years));
        println!("{:?}", schedule_creator.get_schedule(11, Lockup::For1Year_1YearLinear));
    }

    #[test]
    fn test_schedule_creator_real() {
        let schedule_creator = ScheduleCreator::new(false);
        println!("{:?}", schedule_creator.get_schedule(1_000_000, Lockup::NoLockup));
        println!("{:?}", schedule_creator.get_schedule(1_000_000, Lockup::For4Years));
        println!("{:?}", schedule_creator.get_schedule(1_000_000, Lockup::For1Year_1YearLinear));
    }

    #[test]
    fn test_date() {
        let today = Utc::today();
        let curr = today.naive_utc().and_hms(0, 0, 0);
        let curr = Utc.ymd(2022, 1, 31).and_hms(0, 0, 0);

        println!("Today: {}", today);
        println!("Current:    {} {}", curr, curr.timestamp());

        for i in 1..=12 {
            let curr2 = shift_months(curr, i);
            println!("Current {:2}: {} {}", i, curr2, curr2.timestamp());
        }
    }
}
