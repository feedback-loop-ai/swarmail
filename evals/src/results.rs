//! Select a closed version once, then decode the original bytes, preserving duplicates.
<<<<<<< HEAD
use crate::runner::TrialResult;
use serde::{
    Deserialize,
    de::{self, IgnoredAny, MapAccess, Visitor},
};
=======
use crate::runner::{TrialResult, TrialResultV1};
use serde::{
    Deserialize, Serialize,
    de::{self, IgnoredAny, MapAccess, Visitor},
};
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ResultRecord {
    V1(TrialResultV1),
    V2(Box<TrialResult>),
}
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
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
<<<<<<< HEAD
pub fn read(bytes: &[u8]) -> Result<TrialResult, String> {
    crate::provenance::depth(bytes)?;
    let Version(version) =
        serde_json::from_slice::<Version>(bytes).map_err(|_| "invalid result record")?;
    match version {
=======
pub fn read(bytes: &[u8]) -> Result<ResultRecord, String> {
    crate::provenance::depth(bytes)?;
    let Version(version) = serde_json::from_slice::<Version>(bytes).map_err(|e| e.to_string())?;
    match version {
        1 => {
            let row: TrialResultV1 = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            row.validate()?;
            Ok(ResultRecord::V1(row))
        }
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
        2 => {
            if bytes
                .len()
                .saturating_add(usize::from(!bytes.ends_with(b"\n")))
                > crate::provenance::RESULT_LIMIT
            {
                return Err("result exceeds 64 KiB".into());
            }
<<<<<<< HEAD
            let row: TrialResult =
                serde_json::from_slice(bytes).map_err(|_| "invalid result record")?;
            row.validate()?;
            Ok(row)
        }
        _ => Err("schema_version unsupported; expected 2".into()),
=======
            let row: TrialResult = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            row.validate()?;
            Ok(ResultRecord::V2(Box::new(row)))
        }
        _ => Err("unsupported result schema".into()),
    }
}
impl ResultRecord {
    pub fn execution(&self) -> TrialResultV1 {
        match self {
            Self::V1(r) => r.clone(),
            Self::V2(r) => TrialResultV1 {
                schema_version: 1,
                trial_id: r.trial_id.clone(),
                task: r.task.clone(),
                agent: r.agent.clone(),
                outcome: r.outcome,
                failure_class: r.failure_class,
                wall_time_ms: r.wall_time_ms,
                agent_exit_status: r.agent_exit_status,
                agent_exits: r.agent_exits.clone(),
                confinement: r.confinement.clone(),
            },
        }
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    }
}
