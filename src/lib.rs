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
use std::io;
use std::io::prelude::*;
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
    pub dtstart: Option<DateTime<Utc>>,
    pub dtend: Option<DateTime<Utc>>,
    pub dtstamp: Option<DateTime<Utc>>,
    pub uid: Option<String>,
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
        }
    }
}

///store the iCalendar and add events from struct `Events`.
#[derive(Clone)]
pub struct Calendar {
    pub prodid: String,
    pub version: String,
    pub calscale: String,
    pub method: String,
    pub x_wr_calname: String,
    pub x_wr_timezone: String,
    pub events: Vec<Event>,
}

macro_rules! assign_if_ok {
    ($lvalue:expr, $rvalue:expr) => {
        if let Ok(rvalue_ok) = $rvalue {
            $lvalue = Some(rvalue_ok);
        }
    };
}

impl Calendar {
    ///Request HTTP or HTTPS to iCalendar url.
    pub async fn new(url: &str) -> anyhow::Result<Calendar> {
        let data = reqwest::get(url)
            .await
            .context("Could not make request")?
            .text()
            .await
            .context("Could not read response")?;
        Self::new_from_data(&data)
    }

    ///Create a `Calendar` from text in memory.
    pub fn new_from_data(data: &str) -> anyhow::Result<Calendar> {
        println!("raw: {data}");
        let text_data = data.lines().collect::<Vec<_>>();
        let mut struct_even: Vec<Event> = Vec::new();

        let mut even_temp = Event::empty();
        let mut prodid = String::new();
        let mut version = String::new();
        let mut calscale = String::new();
        let mut method = String::new();
        let mut x_wr_calname = String::new();
        let mut x_wr_timezone = String::new();

        for i in text_data {
            let kv = i.splitn(2, ':').collect::<Vec<_>>();

            if kv.len() != 2 {
                // FIXME: it would be nice if multiline values were supported,
                // which would require a different parsing strategy than
                // assuming KEY:VALUE\n as we're currently doing.
                log::warn!("Could not find ':' in '{}, discarding line", i);
                continue;
            };

            let key_cal = kv[0];
            let key_cal = key_cal.split_once(';').map(|(val, _)| val).unwrap_or(key_cal);
            let value_cal = kv[1].to_string();

            log::trace!("processing {}:{}", &key_cal, &value_cal);

            match key_cal {
                "PRODID" => {
                    prodid = value_cal;
                }
                "VERSION" => {
                    version = value_cal;
                }
                "CALSCALE" => {
                    calscale = value_cal;
                }
                "METHOD" => {
                    method = value_cal;
                }
                "X-WR-CALNAME" => {
                    x_wr_calname = value_cal;
                }
                "X-WR-TIMEZONE" => {
                    x_wr_timezone = value_cal;
                }
                "UID" => {
                    even_temp.uid = Some(value_cal);
                }
                "DESCRIPTION" => {
                    even_temp.description = Some(value_cal);
                }
                "LOCATION" => {
                    even_temp.location = Some(value_cal);
                }
                "SEQUENCE" => {
                    even_temp.sequence = Some(value_cal.parse::<u32>().unwrap());
                }
                "STATUS" => {
                    even_temp.status = Some(value_cal);
                }
                "SUMMARY" => {
                    even_temp.summary = Some(value_cal);
                }
                "TRANSP" => {
                    even_temp.transp = Some(value_cal);
                }
                "ORGANIZER" => {
                    even_temp.organizer = Some(value_cal);
                }
                "RRULE" => {
                    let mut vals = value_cal.split(';');
                    let freq = {
                        let freq = vals.next().unwrap();
                        if freq.starts_with("FREQ=") {
                            &freq["FREQ=".len()..]
                        } else {
                            println!("Found weird rrule: {}", value_cal);
                            continue;
                        }
                    };
                    let until = vals.next().map(|until| {
                        if until.starts_with("UNTIL=") {
                            match convert_datetime(&freq["UNTIL=".len()..], "%Y%m%dT%H%M%S") {
                                Ok(val) => {
                                    Some(val)
                                }
                                Err(_) => None,
                            }
                        } else {
                            println!("Found weird rrule: {}", value_cal);
                            None
                        }
                    }).flatten();
                    even_temp.repeat = Some(Repeat { freq: freq.to_string(), until });
                }
                "DTSTART" => match convert_datetime(&value_cal, "%Y%m%dT%H%M%S") {
                    Ok(val) => {
                        even_temp.dtstart = Some(val);
                    }
                    Err(_) => (),
                },
                "DTSTART;VALUE=DATE" => {
                    let aux_date = value_cal + "T000000Z";
                    match convert_datetime(&aux_date, "%Y%m%dT%H%M%S") {
                        Ok(val) => {
                            even_temp.dtstart = Some(val);
                        }
                        Err(_) => (),
                    }
                }
                "DTEND" => match convert_datetime(&value_cal, "%Y%m%dT%H%M%S") {
                    Ok(val) => {
                        even_temp.dtend = Some(val);
                    }
                    Err(_) => (),
                },
                "DTEND;VALUE=DATE" => {
                    let time_cal = "T002611Z";
                    let aux_date = value_cal + time_cal;
                    assign_if_ok!(
                        even_temp.dtend,
                        convert_datetime(&aux_date, "%Y%m%dT%H%M%S")
                    );
                }
                "DTSTAMP" => {
                    assign_if_ok!(
                        even_temp.dtstamp,
                        convert_datetime(&value_cal, "%Y%m%dT%H%M%S")
                    );
                }
                "CREATED" => {
                    assign_if_ok!(
                        even_temp.created,
                        convert_datetime(&value_cal, "%Y%m%dT%H%M%S")
                    );
                }
                "LAST-MODIFIED" => {
                    assign_if_ok!(
                        even_temp.last_modified,
                        convert_datetime(&value_cal, "%Y%m%dT%H%M%S")
                    );
                }
                "END" if value_cal == "VEVENT" => {
                    struct_even.push(even_temp.clone());
                }
                other => {
                    println!("unhandled key: {}", other);
                }
            }
        }

        Ok(Calendar {
            prodid,
            version,
            calscale,
            method,
            x_wr_calname,
            x_wr_timezone,
            events: struct_even,
        })
    }
    ///Create your own iCalendar instance
    /// # Create an iCalendar
    /// ```
    /// let mut ical =  Calendar::create(
    ///                       "-//My Business Inc//My Calendar 70.9054//EN",
    ///                       "2.0",
    ///                       "GREGORIAN",
    ///                       "PUBLISH",
    ///                       "example@gmail.com",
    ///                       "America/New_York");
    /// ```
    pub fn create(
        prodid: &str,
        version: &str,
        calscale: &str,
        method: &str,
        x_wr_calname: &str,
        x_wr_timezone: &str,
    ) -> Calendar {
        Calendar {
            prodid: prodid.to_string(),
            version: version.to_string(),
            calscale: calscale.to_string(),
            method: method.to_string(),
            x_wr_calname: x_wr_calname.to_string(),
            x_wr_timezone: x_wr_timezone.to_string(),
            events: vec![],
        }
    }
    ///Add events to the calendar.
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
        write!(writer, "CALSCALE:{}\r\n", &self.calscale)?;
        write!(writer, "VERSION:{}\r\n", &self.version)?;
        write!(writer, "METHOD:{}\r\n", &self.method)?;
        write!(writer, "X-WR-CALNAME:{}\r\n", &self.x_wr_calname)?;
        write!(writer, "X-WR-TIMEZONE:{}\r\n", &self.x_wr_timezone)?;
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
        data.push_str("\r\n");
        data.push_str("CALSCALE:");
        data.push_str(&self.calscale);
        data.push_str("\r\n");
        data.push_str("VERSION:");
        data.push_str(&self.version);
        data.push_str("\r\n");
        data.push_str("METHOD:");
        data.push_str(&self.method);
        data.push_str("\r\n");
        data.push_str("X-WR-CALNAME:");
        data.push_str(&self.x_wr_calname);
        data.push_str("\r\n");
        data.push_str("X-WR-TIMEZONE:");
        data.push_str(&self.x_wr_timezone);
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
