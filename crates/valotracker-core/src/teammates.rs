// "who do they play with" - for one player, scan their recent matches and find
// teammates who keep showing up. throttled HARD (one request at a time with a
// delay between each) so we trickle politely instead of hammering riot, which is
// what gets accounts rate-limited. on-demand / opt-in only.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::auth::Auth;

// how many of the player's recent matches to scan
const MATCHES_TO_SCAN: usize = 15;
// wait between each match-details call - the polite trickle
const THROTTLE: Duration = Duration::from_millis(1500);

// progress / result, streamed back to the ui as it goes
#[derive(Debug, Clone)]
pub enum TeammateMsg {
    Progress { done: usize, total: usize },
    Done(Vec<Teammate>),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Teammate {
    pub puuid: String,
    // "name#tag" when we can get it, otherwise the puuid
    pub name: String,
    // how many of the scanned matches they were on this player's team
    pub games: u32,
}

// kick the analysis off on its own thread. caller polls `rx` for progress.
pub fn spawn_analysis(auth: Auth, target: String, tx: Sender<TeammateMsg>) {
    std::thread::Builder::new()
        .name("vt-teammates".into())
        .spawn(move || {
            if let Err(e) = run(&auth, &target, &tx) {
                let _ = tx.send(TeammateMsg::Failed(e));
            }
        })
        .ok();
}

fn run(auth: &Auth, target: &str, tx: &Sender<TeammateMsg>) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(auth.riot_headers())
        .build()
        .map_err(|e| e.to_string())?;

    // 1. the target's recent competitive match ids
    let hist_url = auth.pvp_url(&format!(
        "/match-history/v1/history/{target}?queue=competitive&endIndex={MATCHES_TO_SCAN}"
    ));
    let hist: serde_json::Value = client
        .get(&hist_url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let match_ids: Vec<String> = hist
        .get("History")
        .and_then(|h| h.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    e.get("MatchID")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_owned())
                })
                .collect()
        })
        .unwrap_or_default();

    let total = match_ids.len();
    // puuid -> (display name, shared-game count)
    let mut tally: HashMap<String, (String, u32)> = HashMap::new();

    for (i, mid) in match_ids.iter().enumerate() {
        let _ = tx.send(TeammateMsg::Progress { done: i, total });

        // trickle: pause between every match (except the first)
        if i > 0 {
            std::thread::sleep(THROTTLE);
        }

        let url = auth.pvp_url(&format!("/match-details/v1/matches/{mid}"));
        let details: serde_json::Value = match client
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
        {
            Ok(v) => v,
            Err(_) => continue, // one bad match shouldnt kill the whole scan
        };

        let players = match details.get("players").and_then(|p| p.as_array()) {
            Some(p) => p,
            None => continue,
        };

        // which team was the target on this game?
        let target_team = players.iter().find_map(|p| {
            if p.get("subject").and_then(|s| s.as_str()) == Some(target) {
                p.get("teamId")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_owned())
            } else {
                None
            }
        });
        let target_team = match target_team {
            Some(t) => t,
            None => continue,
        };

        for p in players {
            let puuid = p.get("subject").and_then(|s| s.as_str()).unwrap_or("");
            if puuid.is_empty() || puuid == target {
                continue;
            }
            // only count actual teammates, not opponents
            if p.get("teamId").and_then(|t| t.as_str()).unwrap_or("") != target_team {
                continue;
            }
            let gn = p.get("gameName").and_then(|x| x.as_str()).unwrap_or("");
            let tl = p.get("tagLine").and_then(|x| x.as_str()).unwrap_or("");
            let name = if gn.is_empty() {
                puuid.to_owned()
            } else if tl.is_empty() {
                gn.to_owned()
            } else {
                format!("{gn}#{tl}")
            };
            let entry = tally.entry(puuid.to_owned()).or_insert((name.clone(), 0));
            if entry.0.is_empty() || entry.0 == *puuid {
                entry.0 = name;
            }
            entry.1 += 1;
        }
    }

    // keep recurring teammates only (2+ shared games), most frequent first
    let mut result: Vec<Teammate> = tally
        .into_iter()
        .filter(|(_, (_, games))| *games >= 2)
        .map(|(puuid, (name, games))| Teammate { puuid, name, games })
        .collect();
    result.sort_by_key(|t| std::cmp::Reverse(t.games));

    let _ = tx.send(TeammateMsg::Done(result));
    Ok(())
}
