//! VALORANT agent UUID → display name lookup.
//!
//! The table is hardcoded for zero network overhead.  For a fully dynamic
//! solution fetch from `https://valorant-api.com/v1/agents` at startup.
//! UUIDs are stored in lowercase and matched case-insensitively.

/// Hardcoded agent UUID → display name map.
/// UUIDs are lowercase; matching is done case-insensitively via [`resolve_agent_name`].
const AGENT_MAP: &[(&str, &str)] = &[
    ("e370fa57-4757-3604-3648-499e1f642d3f", "Gekko"),
    ("dade69b4-4f5a-8528-247b-219e5a1facd6", "Fade"),
    ("5f8d3a7f-467b-97f3-062c-13acf203c006", "Breach"),
    ("cc8b64c8-4b25-4ff9-6e7f-37b4da43d235", "Deadlock"),
    ("f94c3b30-42be-e959-889c-5aa313dba261", "Raze"),
    ("22697a3d-45bf-8dd7-4fec-84a9e28c69d7", "Chamber"),
    ("601dbbe7-43ce-be57-2a40-4abd24953621", "KAY/O"),
    ("6f2a04ca-43e0-be17-7f36-b3908627744d", "Skye"),
    ("117ed9e3-49f3-6512-3ccf-0cada7e3823b", "Cypher"),
    ("320b2a48-4d9b-a075-30f1-1f93a9b638fa", "Sova"),
    ("1dbf2edd-4729-0984-3115-daa5eed44993", "Killjoy"),
    ("95b78ed7-4637-86d9-7e41-71ba8c293152", "Harbor"),
    ("eb93336a-449b-9c1b-0a54-a891f7921d69", "Phoenix"),
    ("41fb69c1-4189-7b37-f117-bcaf1e96f1bf", "Astra"),
    ("9f0d8ba9-4140-b941-57d3-a7ad57c6b417", "Brimstone"),
    ("0e38b510-41a8-5780-5e8f-568b2a4f2d6c", "Iso"),
    ("bb2a4828-46eb-8cd1-e765-15848195d751", "Neon"),
    ("8e253930-4c05-31dd-1b6c-968525494517", "Omen"),
    ("1e58de9c-4950-5125-93e9-a0aee9f98746", "Clove"),
    ("dea89a98-4c10-36a5-8d26-9db77a2b7a5e", "Waylay"),
    ("569fdd95-4d10-43ab-ca70-79becc718b46", "Sage"),
    ("a3bfb853-43b2-7238-a4f1-ad90e9e46bcc", "Reyna"),
    ("707eab51-4836-f488-046a-cda6bf494859", "Viper"),
    ("8370acf1-4667-35f5-310b-958a3defdba3", "Tejo"),
    ("7e73a75c-465b-8870-5647-9950a4788de1", "Vyse"),
    ("efba5359-4016-a1e5-7626-b1ae976b0e68", "Yoru"),
    ("add6443a-41bd-e414-f6ad-e58d267f4e95", "Jett"),
    ("f0767e4e-97f1-4d22-b7b2-08a7aef7afc5", "Smoked Out"), // Clove variant
];

/// Resolve a VALORANT agent UUID to its display name.
///
/// Returns `"Unknown"` for UUIDs not yet in the table (e.g., newly released
/// agents before the next ValoTracker update).
pub fn resolve_agent_name(character_id: &str) -> String {
    let id_lower = character_id.to_lowercase();
    AGENT_MAP
        .iter()
        .find(|(uuid, _)| *uuid == id_lower)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| "Unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_agents_resolve() {
        assert_eq!(
            resolve_agent_name("707eab51-4836-f488-046a-cda6bf494859"),
            "Viper"
        );
        assert_eq!(
            resolve_agent_name("add6443a-41bd-e414-f6ad-e58d267f4e95"),
            "Jett"
        );
        assert_eq!(
            resolve_agent_name("569fdd95-4d10-43ab-ca70-79becc718b46"),
            "Sage"
        );
    }

    #[test]
    fn unknown_agent_returns_unknown() {
        assert_eq!(
            resolve_agent_name("00000000-0000-0000-0000-000000000000"),
            "Unknown"
        );
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(
            resolve_agent_name("707EAB51-4836-F488-046A-CDA6BF494859"),
            "Viper"
        );
    }

    /// Guard against duplicate UUIDs being accidentally re-introduced.
    #[test]
    fn agent_uuids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for (uuid, name) in AGENT_MAP {
            assert!(
                seen.insert(*uuid),
                "Duplicate UUID {uuid} in agent table (agent: {name})"
            );
        }
    }
}
