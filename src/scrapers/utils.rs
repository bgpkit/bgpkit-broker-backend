use chrono::Datelike;

/// Returns true if the year is a leap-year, as naively defined in the Gregorian calendar.
#[inline]
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// If the day lies within the month, this function has no effect. Otherwise, it shifts
// day backwards to the final day of the month.
// XXX: No attempt is made to handle days outside the 1-31 range.
#[inline]
fn normalise_day(year: i32, month: u32, day: u32) -> u32 {
    if day <= 28 {
        day
    } else if month == 2 {
        28 + is_leap_year(year) as u32
    } else if day == 31 && (month == 4 || month == 6 || month == 9 || month == 11) {
        30
    } else {
        day
    }
}

pub fn shift_months<D: Datelike>(date: D, months: i32) -> D {
    let mut year = date.year() + (date.month() as i32 + months) / 12;
    let mut month = (date.month() as i32 + months) % 12;
    let mut day = date.day();

    if month < 1 {
        year -= 1;
        month += 12;
    }

    day = normalise_day(year, month as u32, day);

    // This is slow but guaranteed to succeed (short of interger overflow)
    if day <= 28 {
        date.with_day(day)
            .unwrap()
            .with_month(month as u32)
            .unwrap()
            .with_year(year)
            .unwrap()
    } else {
        date.with_day(1)
            .unwrap()
            .with_month(month as u32)
            .unwrap()
            .with_year(year)
            .unwrap()
            .with_day(day)
            .unwrap()
    }
}

