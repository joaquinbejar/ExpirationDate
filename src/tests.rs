#[cfg(test)]
mod tests {
    use crate::ExpirationDate;
    use crate::conventions::{DayCount, Actual360, Thirty360US};
    use chrono::{Duration, TimeZone, Utc, Datelike};
    use positive::Positive;
    use positive::constants::DAYS_IN_A_YEAR;

    #[test]
    fn test_actual_365_fixed() {
        let exp = ExpirationDate::Days(DAYS_IN_A_YEAR);
        let years = exp.get_years().unwrap();
        assert!((years.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_actual_360() {
        let exp = ExpirationDate::Days(Positive::new(360.0).unwrap());
        let years = exp.get_years_with_convention(Actual360).unwrap();
        assert!((years.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_thirty_360_us() {
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        
        let conv = Thirty360US;
        let days = conv.day_count(&start, &end).unwrap();
        assert_eq!(days, 360.0);
    }

    #[test]
    fn test_parsing_hft_format() {
        let s = "20251231";
        let exp = ExpirationDate::from_string(s).unwrap();
        match exp {
            ExpirationDate::DateTime(dt) => {
                assert_eq!(dt.year(), 2025);
                assert_eq!(dt.month(), 12);
                assert_eq!(dt.day(), 31);
            },
            _ => panic!("Expected DateTime, got {:?}", exp),
        }
    }
}
