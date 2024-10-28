//! # web_ical
//!
//! `web_ical` is an esay iCalendar Rust library. It’s goals are to read and write ics web files (Google Calendar, Airbnb Calendar and more) data in a developer-friendly way.
//!
//! # Examples 1
//! ```
//! extern crate web_ical;
//!
//!use web_ical::Calendar;
//!
//!fn main() {
//!    let icals = Calendar::new("http://ical.mac.com/ical/US32Holidays.ics").unwrap();
//!
//!    for ical in &icals.events{
//!         println!("Event: {}", ical.summary);
//!         println!("Started: {}", ical.dtstart.format("%a, %e %b %Y - %T"));
//!    }
//!}
//! ```
//! # Examples 2
//! ```
//! extern crate web_ical;
//!
//!use web_ical::Calendar;
//!
//!fn main() {
//!    let icals = Calendar::new("http://ical.mac.com/ical/US32Holidays.ics");
//!     println!("UTC now is: {}", icals.events[0].dtstart);
//!     println!("UTC now in RFC 2822 is: {}", icals.events[0].dtstart.to_rfc2822());
//!     println!("UTC now in RFC 3339 is: {}", icals.events[0].dtstart.to_rfc3339());
//!     println!("UTC now in a custom format is: {}", icals.events[0].dtstart.format("%a %b %e %T %Y"));
//!}
//! ```
extern crate chrono;

use anyhow::Context;
use chrono::Utc;
use chrono::{DateTime, NaiveDateTime};
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Cursor, ErrorKind};
use std::path::Path;

///Convert datetime string to [`DateTime`](https://docs.rs/chrono/0.4.7/chrono/struct.DateTime.html)
///
/// # Examples
///
/// ```
/// let result_obj_aux: Result<DateTime<Utc>, String>;
/// result_obj_aux = convert_datetime("20190522T232701Z", "%Y%m%dT%H%M%SZ".to_string());
/// match result_obj_aux{
///     Ok(val) => {
///             println!("{}", val);
///         },
///     Err(_) => (),
///}
///```
fn convert_datetime(value: &str, format: &str) -> anyhow::Result<DateTime<Utc>> {
    let no_timezone_aux = NaiveDateTime::parse_from_str(value, format)?;
    Ok(DateTime::from_utc(no_timezone_aux, Utc))
}

///store all events from iCalendar.
#[derive(Clone)]
// You should have called it Event, as it is only one event
pub struct Event {
    pub dtstamp: Option<DateTime<Utc>>,
    pub uid: Option<String>,
    pub dtstart: Option<DateTime<Utc>>,
    pub dtend: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub organizer: Option<String>,
    pub sequence: Option<u32>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub transp: Option<String>,
    pub repeat: Option<Repeat>,
    pub class: Option<String>,
    pub geo: Option<String>,
    // pub last_mod: Option<String>,
    pub priority: Option<String>,
    pub recur_id: Option<String>,
    pub url: Option<String>,
    // missing: duration support,
    /*
    attach / attendee / categories / comment /
                  contact / exdate / rstatus / related /
                  resources / rdate / x-prop / iana-prop
     */
}

impl Event {
    #[inline]
    fn check_consistency(&self, cal_has_method: bool) -> bool {
        // if no method is specified on the calendar object, all of it's events have to specify a dtstart
        self.dtstamp.is_some() && self.uid.is_some() && (cal_has_method || self.dtstart.is_some())
    }

    fn set_dt_start(&mut self, val: &str) -> anyhow::Result<()> {
        if self.dtstart.is_some() {
            panic!("Dtstart may not be specified more than once");
        }
        self.dtstart = Some(convert_datetime(
            &format!("{val}T000000Z"),
            "%Y%m%dT%H%M%S",
        )?);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Repeat {
    pub freq: String,
    pub until: Option<DateTime<Utc>>,
}

impl Event {
    ///Check if the events is all day.
    pub fn is_all_day(&self) -> Option<bool> {
        self.dtstart
            .as_ref()
            .zip(self.dtend.as_ref())
            .map(|(start, end)| end.signed_duration_since(start).num_hours() >= 24)
    }
    pub fn empty() -> Event {
        Event {
            dtstart: None,
            dtend: None,
            dtstamp: None,
            uid: None,
            created: None,
            description: None,
            last_modified: None,
            location: None,
            organizer: None,
            sequence: None,
            status: None,
            summary: None,
            transp: None,
            repeat: None,
            class: None,
            geo: None,
            priority: None,
            recur_id: None,
            url: None,
        }
    }
}

/// store the iCalendar and add events from struct `Events`.
#[derive(Clone)]
pub struct Calendar {
    pub name: Option<String>,
    pub prodid: String,
    pub version: String,
    pub calscale: Option<String>,
    pub method: Option<String>,
    pub x_wr_calname: Option<String>,
    pub x_wr_timezone: Option<String>,
    pub events: Vec<Event>,
}

macro_rules! assign_if_ok {
    ($lvalue:expr, $rvalue:expr) => {
        if let Ok(rvalue_ok) = $rvalue {
            $lvalue = Some(rvalue_ok);
        }
    };
}

struct CalendarBuilder {
    prodid: Option<String>,
    version: Option<String>,
    calscale: Option<String>,
    method: Option<String>,
    x_wr_timezone: Option<String>,
    x_wr_calname: Option<String>,
    name: Option<String>,
    events: Vec<Event>,
}

fn parse_cal(raw: &str) -> anyhow::Result<Calendar> {
    let mut raw = Cursor::new(raw);
    let mut buf = String::new();

    raw.read_line(&mut buf)?;
    // FIXME: handle this gracefully
    assert_eq!(&buf, "BEGIN:VCALENDAR\r\n");

    // FIXME: put all this into a builder struct!
    let mut prodid = None;
    let mut version = None;
    let mut calscale = None;
    let mut method = None;
    let mut name = None;
    let mut x_wr_calname = None;
    let mut x_wr_timezone = None;

    let mut events = vec![];
    loop {
        buf.clear();
        if raw.read_line(&mut buf)? == 0 {
            return Err(anyhow::Error::new(io::Error::from(
                ErrorKind::UnexpectedEof,
            )));
        }
        // remove the new line character
        buf.pop();
        if &buf == "END:VCALENDAR" {
            // FIXME: error if cursor has more data to read
            return Ok(Calendar {
                prodid: prodid.expect("a calendar needs a prodid"),
                version: version.expect("a calendar needs a version"),
                calscale,
                method,
                x_wr_calname,
                x_wr_timezone,
                events,
                name,
            });
        }
        let (key, value) = if let Some(kv) = buf.split_once(':') {
            kv
        } else {
            println!("Found bad line: {}", buf);
            continue;
        };
        match key {
            "NAME" => {
                name = Some(value.to_string());
            }
            "PRODID" => {
                assert!(prodid.is_none());
                prodid = Some(value.to_string());
            }
            "VERSION" => {
                assert!(version.is_none());
                version = Some(value.to_string());
            }
            "CALSCALE" => {
                calscale = Some(value.to_string());
            }
            "METHOD" => {
                method = Some(value.to_string());
            }
            "X-WR-CALNAME" => {
                x_wr_calname = Some(value.to_string());
            }
            "X-WR-TIMEZONE" => {
                x_wr_timezone = Some(value.to_string());
            }
            "BEGIN" => {
                if value == "VEVENT" {
                    events.push(parse_event(&mut raw)?);
                } else {
                    // FIXME: todo support this!
                }
            }
            _ => println!("unknown calendar key value pair \"{key}\": \"{value}\""),
        }
    }
}

fn parse_event(raw: &mut Cursor<&str>) -> anyhow::Result<Event> {
    let mut buf = String::new();
    let mut ev = Event::empty();
    loop {
        buf.clear();
        if raw.read_line(&mut buf)? == 0 {
            return Err(anyhow::Error::new(io::Error::from(
                ErrorKind::UnexpectedEof,
            )));
        }
        // remove the new line character
        buf.pop();
        if &buf == "END:VEVENT" {
            return Ok(ev);
        }
        let (key, value) = if let Some(kv) = buf.split_once(':') {
            kv
        } else {
            println!("Found bad line: {}", buf);
            continue;
        };
        // FIXME: properly handle multi part keys
        let key = key.split(';').next().unwrap_or(key);
        match key {
            "CLASS" => {
                ev.class = Some(value.to_string());
            }
            "GEO" => {
                ev.geo = Some(value.to_string());
            }
            "PRIORITY" => {
                ev.priority = Some(value.to_string());
            }
            "RECUR-ID" => {
                ev.recur_id = Some(value.to_string());
            }
            "URL" => {
                ev.url = Some(value.to_string());
            }
            "UID" => {
                ev.uid = Some(value.to_string());
            }
            "DESCRIPTION" => {
                ev.description = Some(value.to_string());
            }
            "LOCATION" => {
                ev.location = Some(value.to_string());
            }
            "SEQUENCE" => {
                ev.sequence = Some(value.parse::<u32>().unwrap());
            }
            "STATUS" => {
                ev.status = Some(value.to_string());
            }
            "SUMMARY" => {
                ev.summary = Some(value.to_string());
            }
            "TRANSP" => {
                ev.transp = Some(value.to_string());
            }
            "ORGANIZER" => {
                ev.organizer = Some(value.to_string());
            }
            "RRULE" => {
                let mut vals = value.split(';');
                // FIXME: can we trust on this order always being this way?
                let freq = {
                    let freq = vals.next().unwrap();
                    if freq.starts_with("FREQ=") {
                        &freq["FREQ=".len()..]
                    } else {
                        println!("Found weird rrule: {}", value);
                        continue;
                    }
                };
                let until = vals
                    .next()
                    .map(|until| {
                        if until.starts_with("UNTIL=") {
                            match convert_datetime(&freq["UNTIL=".len()..], "%Y%m%dT%H%M%S") {
                                Ok(val) => Some(val),
                                Err(_) => None,
                            }
                        } else {
                            println!("Found weird rrule: {}", value);
                            None
                        }
                    })
                    .flatten();
                ev.repeat = Some(Repeat {
                    freq: freq.to_string(),
                    until,
                });
            }
            "DTSTART" => match convert_datetime(&value, "%Y%m%dT%H%M%S") {
                Ok(val) => {
                    ev.dtstart = Some(val);
                }
                Err(_) => (),
            },
            "DTEND" => match convert_datetime(&value, "%Y%m%dT%H%M%S") {
                Ok(val) => {
                    ev.dtend = Some(val);
                }
                Err(_) => (),
            },
            "DTSTAMP" => {
                assign_if_ok!(ev.dtstamp, convert_datetime(&value, "%Y%m%dT%H%M%S"));
            }
            "CREATED" => {
                assign_if_ok!(ev.created, convert_datetime(&value, "%Y%m%dT%H%M%S"));
            }
            "LAST-MODIFIED" => {
                assign_if_ok!(ev.last_modified, convert_datetime(&value, "%Y%m%dT%H%M%S"));
            }
            other => {
                println!("unhandled key, value: \"{}\": \"{}\"", other, value);
            }
        }
    }
}

impl Calendar {
    /// Request HTTP or HTTPS to iCalendar url.
    pub async fn new(url: &str) -> anyhow::Result<Calendar> {
        let data = reqwest::get(url)
            .await
            .context("Could not make request")?
            .text()
            .await
            .context("Could not read response")?;
        Self::new_from_data(&data)
    }

    /// Create a `Calendar` from text in memory.
    pub fn new_from_data(data: &str) -> anyhow::Result<Calendar> {
        parse_cal(data)
    }
    /// Add events to the calendar.
    ///
    /// # Add events
    /// ```
    /// let mut start_cal:  DateTime<Utc> = Utc::now();
    //  let date_tz: DateTime<Utc> = Utc::now();
    /// let start = date_tz.checked_add_signed(Duration::days(2));
    ///
    /// match start {
    ///       Some(x) => {
    ///              start_cal = x;
    ///       },
    ///       None => ()
    /// }
    /// let own_event = Events{
    ///                    
    ///                    dtstart:        start_cal,
    ///                    dtend:          start_cal,
    ///                    dtstamp:        date_tz,
    ///                    uid:            "786566jhjh5546@google.com".to_string(),
    ///                    created:        date_tz,
    ///                    description:    "The description".to_string(),
    ///                    last_modified:  date_tz,
    ///                    location:       "Homestead FL".to_string(),
    ///                    sequence:       0,
    ///                    status:         "CONFIRMED".to_string(),
    ///                    summary:        "My business (Not available)".to_string(),
    ///                    transp:         "OPAQUE".to_string()
    ///
    ///    };
    /// let mut ical =  Calendar::create(
    ///                       "-//My Business Inc//My Calendar 70.9054//EN",
    ///                       "2.0",
    ///                       "GREGORIAN",
    ///                       "PUBLISH",
    ///                       "example@gmail.com",
    ///                       "America/New_York");
    ///
    /// ical.add_event(own_event);
    /// println!("{}", ical.events[0].summary);
    ///
    /// ```
    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Export iCalendar to any `Write` implementer.
    ///
    /// # iCalendar to stdout
    /// ```
    /// ical.export_to(&mut std::io::stdout()).expect("Could not export to stdout");
    /// ```
    ///
    pub fn export_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write!(writer, "BEGIN:VCALENDAR\r\n")?;
        write!(writer, "PRODID:{}\r\n", &self.prodid)?;
        if let Some(scale) = self.calscale.as_ref() {
            write!(writer, "CALSCALE:{}\r\n", scale)?;
        }
        write!(writer, "VERSION:{}\r\n", &self.version)?;
        if let Some(method) = self.method.as_ref() {
            write!(writer, "METHOD:{}\r\n", method)?;
        }
        if let Some(val) = self.x_wr_calname.as_ref() {
            write!(writer, "X-WR-CALNAME:{}\r\n", val)?;
        }
        if let Some(tz) = self.x_wr_timezone.as_ref() {
            write!(writer, "X-WR-TIMEZONE:{}\r\n", tz)?;
        }
        for i in &self.events {
            write!(writer, "BEGIN:VEVENT\r\n")?;
            write!(
                writer,
                "DTSTART:{}\r\n",
                &i.dtstart.as_ref().unwrap().format("%Y%m%dT%H%M%SZ")
            )?;
            write!(
                writer,
                "DTEND:{}\r\n",
                &i.dtend.as_ref().unwrap().format("%Y%m%dT%H%M%SZ")
            )?;
            write!(
                writer,
                "DTSTAMP:{}\r\n",
                &i.dtstamp.as_ref().unwrap().format("%Y%m%dT%H%M%SZ")
            )?;
            write!(writer, "UID:{}\r\n", &i.uid.as_ref().unwrap())?;
            write!(
                writer,
                "CREATED:{}\r\n",
                &i.created.as_ref().unwrap().format("%Y%m%dT%H%M%SZ")
            )?;
            write!(
                writer,
                "DESCRIPTION:{}\r\n",
                &i.description.as_ref().unwrap()
            )?;
            write!(
                writer,
                "LAST-MODIFIED:{}\r\n",
                &i.last_modified.as_ref().unwrap().format("%Y%m%dT%H%M%SZ")
            )?;
            write!(writer, "LOCATION:{}\r\n", &i.location.as_ref().unwrap())?;
            write!(writer, "SEQUENCE:{}\r\n", &i.sequence.as_ref().unwrap())?;
            write!(writer, "STATUS:{}\r\n", &i.status.as_ref().unwrap())?;
            write!(writer, "SUMMARY:{}\r\n", &i.summary.as_ref().unwrap())?;
            write!(writer, "TRANSP:{}\r\n", &i.transp.as_ref().unwrap())?;
            write!(writer, "END:VEVENT\r\n")?;
        }
        write!(writer, "END:VCALENDAR")?;
        Ok(())
    }

    ///Export iCalendar to a file.
    ///
    /// # iCalendar to a file
    /// ```
    ///  match ical.export_ics("ical.ics"){
    ///        Ok(_) => println!("OK"),
    ///        Err(_) => panic!("Err")
    ///    };
    /// ```
    pub fn export_ics(&self, path: &str) -> io::Result<bool> {
        let mut data = "BEGIN:VCALENDAR\r\n".to_string();
        let path = Path::new(path);
        let mut f = File::create(&path)?;
        data.push_str("PRODID:");
        data.push_str(&self.prodid);
        if let Some(scale) = self.calscale.as_ref() {
            data.push_str("\r\n");
            data.push_str("CALSCALE:");
            data.push_str(scale);
        }
        data.push_str("\r\n");
        data.push_str("VERSION:");
        data.push_str(&self.version);
        if let Some(method) = self.method.as_ref() {
            data.push_str("\r\n");
            data.push_str("METHOD:");
            data.push_str(method);
        }
        if let Some(calname) = self.x_wr_calname.as_ref() {
            data.push_str("\r\n");
            data.push_str("X-WR-CALNAME:");
            data.push_str(calname);
        }
        if let Some(tz) = self.x_wr_timezone.as_ref() {
            data.push_str("\r\n");
            data.push_str("X-WR-TIMEZONE:");
            data.push_str(tz);
        }
        data.push_str("\r\n");
        for i in &self.events {
            data.push_str("BEGIN:VEVENT\r\n");
            data.push_str("DTSTART:");
            data.push_str(
                &i.dtstart
                    .as_ref()
                    .unwrap()
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
            data.push_str("\r\n");
            data.push_str("DTEND:");
            data.push_str(
                &i.dtend
                    .as_ref()
                    .unwrap()
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
            data.push_str("\r\n");
            data.push_str("DTSTAMP:");
            data.push_str(
                &i.dtstamp
                    .as_ref()
                    .unwrap()
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
            data.push_str("\r\n");
            data.push_str("UID:");
            data.push_str(&i.uid.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("CREATED:");
            data.push_str(
                &i.created
                    .as_ref()
                    .unwrap()
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
            data.push_str("\r\n");
            data.push_str("DESCRIPTION:");
            data.push_str(&i.description.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("LAST-MODIFIED:");
            data.push_str(
                &i.last_modified
                    .as_ref()
                    .unwrap()
                    .format("%Y%m%dT%H%M%SZ")
                    .to_string(),
            );
            data.push_str("\r\n");
            data.push_str("LOCATION:");
            data.push_str(&i.location.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("SEQUENCE:");
            data.push_str(&i.sequence.as_ref().unwrap().to_string());
            data.push_str("\r\n");
            data.push_str("STATUS:");
            data.push_str(&i.status.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("SUMMARY:");
            data.push_str(&i.summary.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("TRANSP:");
            data.push_str(&i.transp.as_ref().unwrap());
            data.push_str("\r\n");
            data.push_str("END:VEVENT\r\n");
        }
        data.push_str("END:VCALENDAR");
        match f.write_all(data.as_bytes()) {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}
