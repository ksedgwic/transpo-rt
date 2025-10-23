use chrono::{
    offset::Offset, DateTime as ChronoDateTime, Duration, FixedOffset, LocalResult, NaiveDateTime,
    TimeZone,
};
use openapi_schema::OpenapiSchema;

#[derive(Debug, Clone)]
pub struct DateTime {
    inner: ChronoDateTime<FixedOffset>,
    had_offset: bool,
}

impl DateTime {
    pub fn from_naive_in_timezone(naive: NaiveDateTime, tz: &chrono_tz::Tz) -> Self {
        let offset = match tz.offset_from_local_datetime(&naive) {
            LocalResult::Single(offset) => offset.fix(),
            LocalResult::Ambiguous(earliest, _) => earliest.fix(),
            LocalResult::None => {
                // Fallback to the previous valid offset when the local time does not exist
                // (e.g. during a DST forward transition).
                let previous = naive - chrono::Duration::seconds(1);
                match tz.offset_from_local_datetime(&previous) {
                    LocalResult::Single(offset) | LocalResult::Ambiguous(offset, _) => offset.fix(),
                    LocalResult::None => tz.offset_from_utc_datetime(&previous).fix(),
                }
            }
        };
        DateTime {
            inner: ChronoDateTime::from_local(naive, offset),
            had_offset: true,
        }
    }

    pub fn with_timezone(&self, tz: &chrono_tz::Tz) -> ChronoDateTime<chrono_tz::Tz> {
        if self.had_offset {
            self.inner.with_timezone(tz)
        } else {
            let naive = self.inner.naive_local();
            match tz.from_local_datetime(&naive) {
                LocalResult::Single(dt) => dt,
                LocalResult::Ambiguous(earliest, _) => earliest,
                LocalResult::None => {
                    let previous = naive - Duration::seconds(1);
                    match tz.from_local_datetime(&previous) {
                        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => {
                            dt + Duration::seconds(1)
                        }
                        LocalResult::None => tz.from_utc_datetime(&naive),
                    }
                }
            }
        }
    }

    pub fn naive_local(&self) -> NaiveDateTime {
        self.inner.naive_local()
    }
}

impl From<ChronoDateTime<FixedOffset>> for DateTime {
    fn from(dt: ChronoDateTime<FixedOffset>) -> Self {
        DateTime {
            inner: dt,
            had_offset: true,
        }
    }
}

impl std::string::ToString for DateTime {
    fn to_string(&self) -> String {
        if self.had_offset {
            self.inner
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                .to_string()
        } else {
            self.inner.naive_local().format("%Y-%m-%dT%H:%M:%S").to_string()
        }
    }
}

impl serde::Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> ::serde::Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| DateTime {
                inner: dt,
                had_offset: true,
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").map(|naive| DateTime {
                    inner: ChronoDateTime::from_local(naive, FixedOffset::east(0)),
                    had_offset: false,
                })
            })
            .map_err(|e| serde::de::Error::custom(format!("datetime format not valid: {}", e)))
    }
}

impl OpenapiSchema for DateTime {
    fn generate_schema(
        _spec: &mut openapi::v3_0::Spec,
    ) -> openapi::v3_0::ObjectOrReference<openapi::v3_0::Schema> {
        openapi::v3_0::ObjectOrReference::Object(openapi::v3_0::Schema {
            schema_type: Some("string".into()),
            format: Some("date-time".into()),
            ..Default::default()
        })
    }
}

/// Common fields used by all the siri's Delivery
///
/// Note: it is referenced as `xxxDelivery` in the siri specifications
#[derive(Serialize, Deserialize, OpenapiSchema, Debug)]
pub struct CommonDelivery {
    pub version: String,
    pub response_time_stamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Id of the query
    pub request_message_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<bool>,
}

impl Default for CommonDelivery {
    fn default() -> Self {
        CommonDelivery {
            version: "2.0".to_string(),
            response_time_stamp: chrono::Utc::now().to_rfc3339(),
            // error_condition: None,
            status: Some(true),
            request_message_ref: None,
        }
    }
}
