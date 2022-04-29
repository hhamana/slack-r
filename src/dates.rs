use chrono::{DateTime, Datelike, Local, Weekday};
use log::debug;

const WEEKDAYS: [Weekday; 5] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
];

pub trait IsWeekday {
    fn to_weekday(&self) -> Weekday;
    fn is_weekday(&self) -> bool {
        let target_weekday = self.to_weekday();
        WEEKDAYS.contains(&target_weekday)
    }
}

impl IsWeekday for DateTime<Local> {
    fn to_weekday(&self) -> Weekday {
        self.date().weekday()
    }
}
impl IsWeekday for chrono::NaiveDate {
    fn to_weekday(&self) -> Weekday {
        self.weekday()
    }
}

pub fn validate_time_input(input_time: String) -> Result<(), String> {
    match chrono::NaiveTime::parse_from_str(&input_time, "%H:%M:%S") {
        Ok(_v) => Ok(()),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Takes a date string such as "2020-10-21" and returns a Datetime instance with local timezone and current time.
/// Returns a String as error, so it can be used to validate while invoking as command line argument too
/// Uses `time` only for its time and offset as template for formatting.
pub fn convert_date_string_to_local(
    input_date: &str,
    time: &DateTime<Local>,
) -> Result<DateTime<Local>, String> {
    let input_plus_time = format!("{} {} {}", input_date, time.time(), time.offset());
    debug!("Processing time input as {}", input_plus_time);
    let parsed_date = input_plus_time
        .parse::<DateTime<Local>>()
        .map_err(|_e| format!("Not a date. Example format: {}", time.naive_local().date(),))?;
    debug!("Date successfully parsed");
    Ok(parsed_date)
}

pub fn validate_date_input(input_date: String) -> Result<(), String> {
    let today = Local::now();
    let parsed_date = convert_date_string_to_local(&input_date, &today)?;
    if parsed_date <= today {
        return Err(format!("Date {} must be in the future", input_date));
    }
    let in_120_days = today + chrono::Duration::days(120);
    if parsed_date >= in_120_days {
        return Err(format!(
            "Date {} must not be more than 120 days in the future",
            input_date
        ));
    }
    debug!("Date confirmed valid");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::{Duration, NaiveDate, TimeZone};

    #[test]
    fn local_is_weekday_true() {
        let aware_monday = Local.ymd(2022, 02, 15).and_hms(0, 0, 0); // monday
        assert_eq!(aware_monday.weekday(), Weekday::Tue);

        assert!(aware_monday.is_weekday());
    }

    #[test]
    fn naive_date_is_weekday_false() {
        let naive_sunday = NaiveDate::from_ymd(2022, 02, 13);
        assert_eq!(naive_sunday.weekday(), Weekday::Sun);
        assert!(!naive_sunday.is_weekday());
    }

    #[test]
    fn input_time_validation_fail() {
        let test_input = ["", "noo", "16:00:62", "10:00"];
        for input in test_input {
            assert!(validate_time_input(input.to_string()).is_err());
        }
    }

    #[test]
    fn input_time_validation_success() {
        let test_input = ["10:00:30", "16:00:00"];
        for input in test_input {
            assert!(validate_time_input(input.to_string()).is_ok());
        }
    }

    #[test]
    fn string_to_local_date_conversion_success() {
        let time = Local::now();
        let res = convert_date_string_to_local("2022-01-05", &time);
        assert!(res.is_ok());
        let expected = Local.ymd(2022, 01, 05).and_time(time.time()).unwrap();
        assert_eq!(res.unwrap(), expected);
    }

    #[test]
    fn string_to_local_date_conversion_fail() {
        let time = Local::now();
        for input in [
            "not a date",
            "2022-02-31",
            "20220122",
            "01-22",
            "2022/01/12",
        ] {
            let res = convert_date_string_to_local(input, &time);
            assert!(res.is_err());
        }
    }

    #[test]
    fn validate_date_input_fail_past_date() {
        let d = validate_date_input("2020-01-20".to_string());
        assert!(d.is_err());
        assert_eq!(d.unwrap_err(), "Date 2020-01-20 must be in the future");
    }

    #[test]
    fn validate_date_input_fail_present_date() {
        let today = Local::now().naive_local().date().to_string();
        // assert_eq!(today, "2022-02-18");
        let d = validate_date_input(today.clone());
        assert!(d.is_err());
        assert_eq!(
            d.unwrap_err(),
            format!("Date {today} must be in the future", today = today)
        );
    }

    #[test]
    fn validate_date_input_fail_120_days_ahead() {
        let today = Local::now();
        let in_120_days = today + Duration::days(120);
        let in_120_days_str = in_120_days.naive_local().date().to_string();
        // assert_eq!(in_120_days_str, "2022-06-18");
        let d = validate_date_input(in_120_days_str.clone());
        assert!(d.is_err());
        assert_eq!(
            d.unwrap_err(),
            format!(
                "Date {future} must not be more than 120 days in the future",
                future = in_120_days_str
            )
        );
    }

    #[test]
    fn validate_date_input_success() {
        let today = Local::now();
        let in_10_days = today + Duration::days(10);
        let in_10_days_str = in_10_days.naive_local().date().to_string();
        // assert_eq!(in_10_days_str, "2022-02-28");
        let d = validate_date_input(in_10_days_str.clone());
        assert!(d.is_ok());
    }
}
