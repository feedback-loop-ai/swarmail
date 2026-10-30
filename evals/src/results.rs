//! Select a closed version once, then decode the original bytes, preserving duplicates.
use crate::runner::TrialResult;
use serde::{
    Deserialize,
    de::{self, IgnoredAny, MapAccess, Visitor},
};
struct Version(u32);
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Probe;
        impl<'de> Visitor<'de> for Probe {
            type Value = Version;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("versioned result object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Version, M::Error> {
                let mut version = None;
                while let Some(k) = map.next_key::<String>()? {
                    if k == "schema_version" {
                        if version.is_some() {
                            return Err(de::Error::duplicate_field("schema_version"));
                        }
                        version = Some(map.next_value()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(Version(version.ok_or_else(|| {
                    de::Error::missing_field("schema_version")
                })?))
            }
        }
        d.deserialize_map(Probe)
    }
}
fn select(bytes: &[u8]) -> Result<u32, String> {
    crate::provenance::depth(bytes)?;
    let Version(version) =
        serde_json::from_slice::<Version>(bytes).map_err(|_| "invalid result record")?;
    Ok(version)
}
/// Read one comparison row: schema 3 only, validated. Native, legacy and
/// unknown versions are refused here exactly as `read` refuses schema 3.
pub fn read_comparison(bytes: &[u8]) -> Result<crate::identity::ComparisonRow, String> {
    if select(bytes)? != crate::identity::COMPARISON_SCHEMA {
        return Err(format!(
            "schema_version unsupported; expected {}",
            crate::identity::COMPARISON_SCHEMA
        ));
    }
    let row: crate::identity::ComparisonRow =
        serde_json::from_slice(bytes).map_err(|_| "invalid result record")?;
    row.validate()?;
    Ok(row)
}
pub fn read(bytes: &[u8]) -> Result<TrialResult, String> {
    let version = select(bytes)?;
    match version {
        2 => {
            if bytes
                .len()
                .saturating_add(usize::from(!bytes.ends_with(b"\n")))
                > crate::provenance::RESULT_LIMIT
            {
                return Err("result exceeds 64 KiB".into());
            }
            let row: TrialResult =
                serde_json::from_slice(bytes).map_err(|_| "invalid result record")?;
            row.validate()?;
            Ok(row)
        }
        _ => Err("schema_version unsupported; expected 2".into()),
    }
}
